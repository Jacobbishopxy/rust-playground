//!

use sea_query::*;

pub const PG_BUILDER: Builder = Builder(BuilderType::PG);
pub const MY_BUILDER: Builder = Builder(BuilderType::MY);

fn gen_column_type(c: ColumnDef, col_type: &sqlz::model::ColumnType) -> ColumnDef {
    match col_type {
        sqlz::model::ColumnType::Binary => c.binary(),
        sqlz::model::ColumnType::Bool => c.boolean(),
        sqlz::model::ColumnType::Int => c.integer(),
        sqlz::model::ColumnType::Float => c.float(),
        sqlz::model::ColumnType::Double => c.double(),
        sqlz::model::ColumnType::Date => c.date(),
        sqlz::model::ColumnType::Time => c.time(),
        sqlz::model::ColumnType::DateTime => c.date_time(),
        sqlz::model::ColumnType::Timestamp => c.timestamp(),
        sqlz::model::ColumnType::Char => c.char(),
        sqlz::model::ColumnType::VarChar => c.string(),
        sqlz::model::ColumnType::Text => c.text(),
        sqlz::model::ColumnType::Json => c.json(),
    }
}

fn gen_column(col: &sqlz::model::Column) -> ColumnDef {
    let c = ColumnDef::new(Alias::new(&col.name));
    let c = gen_column_type(c, &col.col_type);
    let c = if col.null.unwrap_or(true) == true {
        c
    } else {
        c.not_null()
    };
    let c = if let Some(ck) = &col.key {
        match ck {
            sqlz::model::ColumnKey::NotKey => c,
            sqlz::model::ColumnKey::Primary => c.primary_key(),
            sqlz::model::ColumnKey::Unique => c.unique_key(),
            sqlz::model::ColumnKey::Multiple => c,
        }
    } else {
        c
    };

    c
}

fn convert_foreign_key_action(
    foreign_key_action: &sqlz::model::ForeignKeyAction,
) -> ForeignKeyAction {
    match foreign_key_action {
        sqlz::model::ForeignKeyAction::Restrict => ForeignKeyAction::Restrict,
        sqlz::model::ForeignKeyAction::Cascade => ForeignKeyAction::Cascade,
        sqlz::model::ForeignKeyAction::SetNull => ForeignKeyAction::SetNull,
        sqlz::model::ForeignKeyAction::NoAction => ForeignKeyAction::NoAction,
        sqlz::model::ForeignKeyAction::SetDefault => ForeignKeyAction::SetDefault,
    }
}

fn convert_index_order(index_order: &sqlz::model::Order) -> IndexOrder {
    match index_order {
        sqlz::model::Order::Asc => IndexOrder::Asc,
        sqlz::model::Order::Desc => IndexOrder::Desc,
    }
}

fn gen_foreign_key(key: &sqlz::model::ForeignKeyCreate) -> ForeignKeyCreateStatement {
    ForeignKey::create()
        .name(&key.name)
        .from(Alias::new(&key.from.table), Alias::new(&key.from.column))
        .to(Alias::new(&key.to.table), Alias::new(&key.to.column))
        .on_delete(convert_foreign_key_action(&key.on_delete))
        .on_update(convert_foreign_key_action(&key.on_update))
}

pub enum BuilderType {
    MY,
    PG,
}

pub struct Builder(pub BuilderType);

// todo: return type
impl Builder {
    pub fn new(builder: BuilderType) -> Self {
        Builder(builder)
    }

    pub fn list_table(&self) -> String {
        match &self.0 {
            BuilderType::MY => "SHOW TABLES;".to_owned(),
            BuilderType::PG => vec![
                r#"SELECT table_name"#,
                r#"FROM information_schema.tables"#,
                r#"WHERE table_schema='public'"#,
                r#"AND table_type='BASE TABLE';"#,
            ]
            .join(" "),
        }
    }

    pub fn create_table(
        &self,
        table: &sqlz::model::TableCreate,
        create_if_not_exists: bool,
    ) -> String {
        let mut s = Table::create();
        s.table(Alias::new(&table.name));

        if create_if_not_exists {
            s.if_not_exists();
        }

        for c in &table.columns {
            s.col(gen_column(c));
        }

        if let Some(f) = &table.foreign_key {
            s.foreign_key(gen_foreign_key(f));
        }

        match &self.0 {
            BuilderType::MY => s.to_string(MysqlQueryBuilder),
            BuilderType::PG => s.to_string(PostgresQueryBuilder),
        }
    }

    pub fn alter_table(&self, table: &sqlz::model::TableAlter) -> Vec<String> {
        let s = Table::alter().table(Alias::new(&table.name));
        let mut alter_series = vec![];

        for a in &table.alter {
            match a {
                sqlz::model::ColumnAlterCase::Add(c) => {
                    alter_series.push(s.clone().add_column(gen_column(c)));
                }
                sqlz::model::ColumnAlterCase::Modify(c) => {
                    alter_series.push(s.clone().modify_column(gen_column(c)));
                }
                sqlz::model::ColumnAlterCase::Rename(c) => {
                    let from_name = Alias::new(&c.from_name);
                    let to_name = Alias::new(&c.to_name);
                    alter_series.push(s.clone().rename_column(from_name, to_name));
                }
                sqlz::model::ColumnAlterCase::Drop(c) => {
                    alter_series.push(s.clone().drop_column(Alias::new(&c.name)));
                }
            }
        }

        alter_series
            .iter()
            .map(|_| match &self.0 {
                BuilderType::MY => s.to_string(MysqlQueryBuilder),
                BuilderType::PG => s.to_string(PostgresQueryBuilder),
            })
            .collect()
    }

    pub fn drop_table(&self, table: &sqlz::model::TableDrop) -> String {
        let s = Table::drop().table(Alias::new(&table.name));

        match &self.0 {
            BuilderType::MY => s.to_string(MysqlQueryBuilder),
            BuilderType::PG => s.to_string(PostgresQueryBuilder),
        }
    }

    pub fn rename_table(&self, table: &sqlz::model::TableRename) -> String {
        let from = Alias::new(&table.from);
        let to = Alias::new(&table.to);
        let s = Table::rename().table(from, to);

        match &self.0 {
            BuilderType::MY => s.to_string(MysqlQueryBuilder),
            BuilderType::PG => s.to_string(PostgresQueryBuilder),
        }
    }

    pub fn truncate_table(&self, table: &sqlz::model::TableTruncate) -> String {
        let s = Table::truncate().table(Alias::new(&table.name));

        match &self.0 {
            BuilderType::MY => s.to_string(MysqlQueryBuilder),
            BuilderType::PG => s.to_string(PostgresQueryBuilder),
        }
    }

    pub fn create_index(&self, index: &sqlz::model::IndexCreate) -> String {
        let mut s = Index::create();
        s = s.name(&index.name).table(Alias::new(&index.table));

        for i in &index.columns {
            match &i.order {
                Some(o) => {
                    s = s.col((Alias::new(&i.name), convert_index_order(o)));
                }
                None => {
                    s = s.col(Alias::new(&i.name));
                }
            }
        }

        match &self.0 {
            BuilderType::MY => s.to_string(MysqlQueryBuilder),
            BuilderType::PG => s.to_string(PostgresQueryBuilder),
        }
    }

    pub fn drop_index(&self, index: &sqlz::model::IndexDrop) -> String {
        let s = Index::drop()
            .name(&index.name)
            .table(Alias::new(&index.table));

        match &self.0 {
            BuilderType::MY => s.to_string(MysqlQueryBuilder),
            BuilderType::PG => s.to_string(PostgresQueryBuilder),
        }
    }

    pub fn create_foreign_key(&self, key: &sqlz::model::ForeignKeyCreate) -> String {
        let s = gen_foreign_key(key);

        match &self.0 {
            BuilderType::MY => s.to_string(MysqlQueryBuilder),
            BuilderType::PG => s.to_string(PostgresQueryBuilder),
        }
    }

    pub fn drop_foreign_key(&self, key: &sqlz::model::ForeignKeyDrop) -> String {
        let s = ForeignKey::drop()
            .name(&key.name)
            .table(Alias::new(&key.table));

        match &self.0 {
            BuilderType::MY => s.to_string(MysqlQueryBuilder),
            BuilderType::PG => s.to_string(PostgresQueryBuilder),
        }
    }

    pub fn select_table(&self, select: &sqlz::model::Select) -> String {
        let mut s = Query::select();

        for c in &select.columns {
            s.column(Alias::new(c));
        }

        s.from(Alias::new(&select.table));

        match &self.0 {
            BuilderType::MY => s.to_string(MysqlQueryBuilder),
            BuilderType::PG => s.to_string(PostgresQueryBuilder),
        }
    }
}

#[cfg(test)]
mod tests_sea {
    use super::*;

    #[test]
    fn test_table_create() {
        let table = sqlz::model::TableCreate {
            name: "test".to_string(),
            columns: vec![
                sqlz::model::Column {
                    name: "id".to_string(),
                    key: Some(sqlz::model::ColumnKey::Primary),
                    ..Default::default()
                },
                sqlz::model::Column {
                    name: "name".to_string(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        println!(
            "{:?}",
            Builder::new(BuilderType::PG).create_table(&table, true)
        );
    }

    #[test]
    fn test_table_alter() {
        let alter = sqlz::model::TableAlter {
            name: "test".to_string(),
            alter: vec![sqlz::model::ColumnAlterCase::Add(sqlz::model::Column {
                name: "name".to_string(),
                ..Default::default()
            })],
        };

        println!("{:?}", Builder::new(BuilderType::PG).alter_table(&alter));
    }

    #[test]
    fn test_index_create() {
        let index = sqlz::model::IndexCreate {
            name: "dev".to_owned(),
            table: "test".to_owned(),
            columns: vec![sqlz::model::IndexCol {
                name: "i".to_owned(),
                ..Default::default()
            }],
        };

        println!("{:?}", Builder::new(BuilderType::PG).create_index(&index));
    }
}