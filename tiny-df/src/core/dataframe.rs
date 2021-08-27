//! Dataframe
//!
//! Core struct of this crate. A `Dataframe` plays as a data structure to hold two dimensional data, heterogeneously.
//! Supporting three kinds of data storage, a `Dataframe` can store data in horizontal, vertical or raw orientation.
//! Gluing external crates such as [arrow-rs](https://github.com/apache/arrow-rs), a `Dataframe` is capable of being
//! converted to other type of sources.
//!
//! Main function:
//! 1. `new`
//! 1. `from_2d_vec`
//! 1. `data`
//! 1. `iloc`
//! 1. `loc`
//! 1. `transpose`
//! 1. `append`
//! 1. `concat`
//! 1. `insert` (multi-dir)
//! 1. `insert_many` (multi-dir)
//! 1. `truncate`
//! 1. `delete` (multi-dir)
//! 1. `delete_many` (multi-dir)
//! 1. `update` (multi-dir)      TODO:
//! 1. `update_many` (multi-dir) TODO:
//! 1. `is_empty`
//! 1. `size`
//! 1. `columns`
//! 1. `columns_name`
//! 1. `indices`
//! 1. `data_orientation`
//! 1. `rename_column`
//! 1. `rename_columns`
//! 1. `replace_index`
//! 1. `replace_indices`
//!

use std::mem;

use serde::{Deserialize, Serialize};

use crate::meta::*;

/// Columns definition
/// 1. D: dynamic column
/// 1. R: reference
enum RefCols<'a> {
    D,
    R(&'a Vec<DataframeColumn>),
}

/// process series (dataframe row) data, e.g. type correction, trim data length
struct DataframeRowProcessor<'a> {
    data: Series,
    columns: RefCols<'a>,
    _cache_col_name: Option<String>,
    _cache_col: Option<DataframeColumn>,
}

impl<'a> DataframeRowProcessor<'a> {
    /// dataframe row processor constructor
    fn new(ref_col: RefCols<'a>) -> Self {
        DataframeRowProcessor {
            data: Vec::new(),
            columns: ref_col,
            _cache_col_name: None,
            _cache_col: None,
        }
    }

    /// check data type, if matching push the data to buf else push None to buf
    fn exec(&mut self, type_idx: usize, data: &mut DataframeData) {
        match self.columns {
            RefCols::D => {
                if type_idx == 0 {
                    // get column name
                    self._cache_col_name = Some(data.to_string());
                    return;
                }
                if type_idx == 1 {
                    // until now (the 2nd cell) we can know the type of this row
                    // create `DataframeColDef` and push to `columns`
                    let cd = DataframeColumn::new(
                        self._cache_col_name.clone().unwrap(),
                        data.as_ref().into(),
                    );

                    self._cache_col = Some(cd);
                }

                // check type and wrap
                let mut tmp = DataframeData::None;
                let value_type: DataType = data.as_ref().into();
                if self._cache_col.as_ref().unwrap().col_type == value_type {
                    mem::swap(&mut tmp, data);
                }

                self.data.push(tmp)
            }
            RefCols::R(r) => {
                // check type and wrap
                let mut tmp = DataframeData::None;
                let value_type: DataType = data.as_ref().into();
                if r.get(type_idx).unwrap().col_type == value_type {
                    mem::swap(&mut tmp, data);
                }

                self.data.push(tmp)
            }
        }
    }

    /// push None to buf
    fn skip(&mut self) {
        self.data.push(DataframeData::None);
    }

    /// get cached column, used for vertical data direction processing
    fn get_cache_col(&self) -> DataframeColumn {
        self._cache_col.clone().unwrap_or_default()
    }
}

/// create an indices for a dataframe
fn create_dataframe_indices(len: usize) -> Vec<DataframeIndex> {
    (0..len)
        .map(|i| DataframeIndex::Id(i as u64))
        .collect::<Vec<_>>()
}

/// Dataframe
/// Core struct of this lib crate
///
/// A dataframe can store three kinds of data, which is determined by its direction:
/// - horizontal presence: each row means one record, certified data size
/// - vertical presence: each column means one record, certified data size
/// - raw: raw data, uncertified data size (each row can have different size)
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Dataframe {
    data: DF,
    columns: Vec<DataframeColumn>,
    indices: Vec<DataframeIndex>,
    data_orientation: DataOrientation,
    size: (usize, usize),
}

/// New dataframe if data_orientation is none
fn new_df_dir_n(data: DF) -> Dataframe {
    Dataframe {
        data,
        ..Default::default()
    }
}

/// New dataframe if data_orientation is horizontal and columns has been given
/// columns length equals dataframe column size
fn new_df_dir_h_col(data: DF, columns: Vec<DataframeColumn>) -> Dataframe {
    let length_of_head_row = columns.len();

    // result init
    let mut res = Vec::new();

    // processing the rest of rows, if exceeded then trim, if insufficient then filling with None
    for mut d in data {
        // each row init a row processor
        let mut processor = DataframeRowProcessor::new(RefCols::R(&columns));

        for i in 0..length_of_head_row {
            match d.get_mut(i) {
                Some(v) => processor.exec(i, v),
                None => processor.skip(),
            }
        }
        res.push(processor.data);
    }

    let length_of_res = res.len();

    Dataframe {
        data: res,
        columns: columns,
        indices: create_dataframe_indices(length_of_res),
        data_orientation: DataOrientation::Horizontal,
        size: (length_of_res, length_of_head_row),
    }
}

/// New dataframe if data_orientation is vertical and columns has been given
/// columns length equals dataframe row size
fn new_df_dir_v_col(data: DF, columns: Vec<DataframeColumn>) -> Dataframe {
    let length_of_head_row = match data.get(0) {
        Some(l) => l.len(),
        None => return Dataframe::default(),
    };
    let length_of_res = columns.len();

    let mut res = Vec::new();

    // processing the rest of rows, if exceeded then trim, if insufficient then filling with None
    for (row_idx, mut d) in data.into_iter().enumerate() {
        let mut processor = DataframeRowProcessor::new(RefCols::R(&columns));
        for i in 0..length_of_head_row {
            match d.get_mut(i) {
                Some(v) => processor.exec(row_idx, v),
                None => processor.skip(),
            }
        }
        res.push(processor.data);
        // break, align to column name
        if row_idx == length_of_res - 1 {
            break;
        }
    }

    Dataframe {
        data: res,
        columns: columns,
        indices: create_dataframe_indices(length_of_head_row),
        data_orientation: DataOrientation::Vertical,
        size: (length_of_res, length_of_head_row),
    }
}

/// New dataframe if data_orientation is horizontal and columns is included in data
fn new_df_dir_h(data: DF) -> Dataframe {
    let mut data_iter = data.iter();
    // take the 1st row as the columns name row
    let columns_name = data_iter
        .next()
        .unwrap()
        .into_iter()
        .map(|d| d.to_string())
        .collect::<Vec<String>>();

    // make sure each row has the same length
    let length_of_head_row = columns_name.len();

    // using the second row to determine columns' type
    let mut column_type: Vec<DataType> = Vec::new();

    // take the 2nd row and determine columns type
    match data_iter.next() {
        Some(vd) => {
            for (i, d) in vd.iter().enumerate() {
                column_type.push(d.into());
                // break, align to column name
                if i == length_of_head_row - 1 {
                    break;
                }
            }
        }
        None => return Dataframe::default(),
    }

    // generate`Vec<DataframeColDef>` and pass it to `new_dataframe_h_dir_col_given`
    let columns = columns_name
        .into_iter()
        .zip(column_type.into_iter())
        .map(|(name, col_type)| DataframeColumn { name, col_type })
        .collect();

    let mut data = data;
    data.remove(0);
    new_df_dir_h_col(data, columns)
}

/// New dataframe if data_orientation is horizontal
fn new_df_dir_v(data: DF) -> Dataframe {
    // take the 1st row length, data row length is subtracted by 1,
    // since the first element must be column name
    let length_of_head_row = data.get(0).unwrap().len();
    if length_of_head_row == 1 {
        return Dataframe::default();
    }

    // init columns & data
    let (mut columns, mut res) = (Vec::new(), Vec::new());

    // unlike `new_df_dir_h_col`, `new_df_dir_v_col` & `new_df_dir_h`,
    // columns type definition is not given, hence needs to iterate through the whole data
    // and dynamically construct it
    for mut d in data.into_iter() {
        let mut processor = DataframeRowProcessor::new(RefCols::D);

        for i in 0..length_of_head_row {
            match d.get_mut(i) {
                Some(v) => processor.exec(i, v),
                None => processor.skip(),
            }
        }
        columns.push(processor.get_cache_col());
        res.push(processor.data);
    }

    let length_of_res = res.len();

    Dataframe {
        data: res,
        columns: columns,
        indices: create_dataframe_indices(length_of_head_row - 1),
        data_orientation: DataOrientation::Vertical,
        size: (length_of_res, length_of_head_row - 1),
    }
}

impl Dataframe {
    /// Dataframe constructor
    /// Accepting tree kinds of data:
    /// 1. in horizontal direction, columns name is the first row
    /// 2. in vertical direction, columns name is the first columns
    /// 3. none direction, raw data
    pub fn new<T, P>(data: T, data_orientation: P) -> Self
    where
        T: Into<DF>,
        P: Into<DataOrientation>,
    {
        let data = data.into();
        if Dataframe::is_empty(&data) {
            return Dataframe::default();
        }
        match data_orientation.into() {
            DataOrientation::Horizontal => new_df_dir_h(data),
            DataOrientation::Vertical => new_df_dir_v(data),
            DataOrientation::Raw => new_df_dir_n(data),
        }
    }

    /// Dataframe constructor
    /// From a 2d vector
    pub fn from_2d_vec<T, P>(data: T, data_orientation: P, columns: Vec<DataframeColumn>) -> Self
    where
        T: Into<DF>,
        P: Into<DataOrientation>,
    {
        let data = data.into();
        if Dataframe::is_empty(&data) || columns.len() == 0 {
            return Dataframe::default();
        }
        match data_orientation.into() {
            DataOrientation::Horizontal => new_df_dir_h_col(data, columns),
            DataOrientation::Vertical => new_df_dir_v_col(data, columns),
            DataOrientation::Raw => new_df_dir_n(data),
        }
    }

    /// get data by numbers of index and column
    pub fn iloc(&self, i: usize, j: usize) -> Option<&DataframeData> {
        match self.data.get(i) {
            Some(r) => match r.get(j) {
                Some(v) => Some(v),
                None => None,
            },
            None => None,
        }
    }

    /// get data by index and column
    pub fn loc<T, S>(&self, i: T, j: S) -> Option<&DataframeData>
    where
        T: Into<DataframeData>,
        S: Into<String>,
    {
        let o_i: DataframeData = i.into();
        let o_j: String = j.into();
        let o_i = self.indices.iter().position(|v| v == &o_i);
        let o_j = self.columns_name().iter().position(|c| c == &o_j);

        match self.data_orientation {
            DataOrientation::Horizontal => match o_i {
                Some(i) => {
                    let v = self.data.get(i).unwrap();
                    match o_j {
                        Some(j) => v.get(j),
                        None => None,
                    }
                }
                None => None,
            },
            DataOrientation::Vertical => match o_j {
                Some(j) => {
                    let v = self.data.get(j).unwrap();
                    match o_i {
                        Some(i) => v.get(i),
                        None => todo!(),
                    }
                }
                None => None,
            },
            DataOrientation::Raw => None,
        }
    }

    /// get dataframe data
    pub fn data(&self) -> &DF {
        &self.data
    }

    /// check if input data is empty
    pub fn is_empty(data: &DF) -> bool {
        if data.is_empty() {
            true
        } else {
            data[0].is_empty()
        }
    }

    /// get dataframe sized
    pub fn size(&self) -> (usize, usize) {
        self.size
    }

    /// get dataframe columns
    pub fn columns(&self) -> &Vec<DataframeColumn> {
        &self.columns
    }

    /// get dataframe columns name
    pub fn columns_name(&self) -> Vec<String> {
        self.columns.iter().map(|c| c.name.to_owned()).collect()
    }

    pub fn indices(&self) -> &Vec<DataframeIndex> {
        &self.indices
    }

    /// get dataframe direction
    pub fn data_orientation(&self) -> &DataOrientation {
        &self.data_orientation
    }

    /// rename specific column name
    pub fn rename_column<T>(&mut self, idx: usize, name: T)
    where
        T: Into<String>,
    {
        self.columns.get_mut(idx).map(|c| c.name = name.into());
    }

    /// rename columns
    pub fn rename_columns<T>(&mut self, names: &[T])
    where
        T: Into<String> + Clone,
    {
        self.columns
            .iter_mut()
            .zip(names.iter())
            .for_each(|(c, n)| c.name = n.clone().into())
    }

    /// replace specific index
    pub fn replace_index<T>(&mut self, idx: usize, data: T)
    where
        T: Into<DataframeData>,
    {
        self.indices.get_mut(idx).map(|i| *i = data.into());
    }

    /// replace indices
    pub fn replace_indices<T>(&mut self, indices: &[T])
    where
        T: Into<DataframeData> + Clone,
    {
        self.indices
            .iter_mut()
            .zip(indices.iter())
            .for_each(|(i, r)| *i = r.clone().into())
    }

    /// transpose dataframe
    pub fn transpose(&mut self) {
        // None direction's data cannot be transposed
        if self.data_orientation == DataOrientation::Raw {
            return;
        }
        let (m, n) = self.size;
        let mut res = Vec::with_capacity(n);
        for j in 0..n {
            let mut row = Vec::with_capacity(m);
            for i in 0..m {
                let mut tmp = DataframeData::None;
                mem::swap(&mut tmp, &mut self.data[i][j]);
                row.push(tmp);
            }
            res.push(row);
        }
        self.data = res;
        self.size = (n, m);
        self.data_orientation = match self.data_orientation {
            DataOrientation::Horizontal => DataOrientation::Vertical,
            DataOrientation::Vertical => DataOrientation::Horizontal,
            DataOrientation::Raw => DataOrientation::Raw,
        }
    }

    /// executed when append a new row to `self.data`
    fn push_indices(&mut self) {
        self.size.0 += 1;
        self.indices.push(DataframeIndex::Id(self.size.0 as u64));
    }

    /// append a new row to `self.data`
    pub fn append(&mut self, data: Series) {
        let mut data = data;

        match self.data_orientation {
            DataOrientation::Horizontal => {
                let mut processor = DataframeRowProcessor::new(RefCols::R(&self.columns));
                for i in 0..self.size.1 {
                    match data.get_mut(i) {
                        Some(v) => processor.exec(i, v),
                        None => processor.skip(),
                    }
                }
                self.data.push(processor.data);
                self.push_indices();
            }
            DataOrientation::Vertical => {
                let mut processor = DataframeRowProcessor::new(RefCols::D);
                // +1 means the first cell representing column name
                for i in 0..self.size.1 + 1 {
                    match data.get_mut(i) {
                        Some(v) => processor.exec(i, v),
                        None => processor.skip(),
                    }
                }
                self.columns.push(processor.get_cache_col());
                self.data.push(processor.data);
                self.push_indices();
            }
            DataOrientation::Raw => {
                self.data.push(data);
            }
        }
    }

    /// concat new data to `self.data`
    pub fn concat(&mut self, data: DF) {
        let mut data = data;

        match self.data_orientation {
            DataOrientation::Horizontal => {
                for row in data {
                    self.append(row);
                }
            }
            DataOrientation::Vertical => {
                for row in data {
                    self.append(row);
                }
            }
            DataOrientation::Raw => {
                self.data.append(&mut data);
            }
        }
    }

    /// executed when insert a new row to `self.data`
    fn insert_indices(&mut self, index: usize, orient: DataOrientation) {
        match orient {
            DataOrientation::Horizontal => {
                self.indices
                    .insert(index, DataframeData::Id(self.size.0 as u64));
                self.size.0 += 1;
            }
            DataOrientation::Vertical => {
                self.indices
                    .insert(index, DataframeData::Id(self.size.1 as u64));
                self.size.1 += 1;
            }
            DataOrientation::Raw => (),
        }
    }

    /// insert a series to a horizontal orientation dataframe
    fn insert_h<T>(&mut self, index: usize, series: Series, orient: T)
    where
        T: Into<DataOrientation>,
    {
        let mut series = series;
        let orient: DataOrientation = orient.into();

        match orient {
            // inserted series as row-wise
            DataOrientation::Horizontal => {
                let mut processor = DataframeRowProcessor::new(RefCols::R(&self.columns));

                for i in 0..self.size.1 {
                    match series.get_mut(i) {
                        Some(v) => processor.exec(i, v),
                        None => processor.skip(),
                    }
                }

                self.data.insert(index, processor.data);
                self.insert_indices(index, orient);
            }
            // inserted series as column-wise
            DataOrientation::Vertical => {
                let mut processor = DataframeRowProcessor::new(RefCols::D);

                for i in 0..self.size.0 + 1 {
                    match series.get_mut(i) {
                        Some(v) => processor.exec(i, v),
                        None => processor.skip(),
                    }

                    if i > 0 {
                        self.data
                            .get_mut(i - 1)
                            .unwrap()
                            .insert(index, processor.data.pop().unwrap());
                    }
                }
                self.columns.insert(index, processor.get_cache_col());
                self.size.1 += 1;
            }
            DataOrientation::Raw => (),
        }
    }

    /// insert a series to a vertical orientation dataframe
    fn insert_v<T>(&mut self, index: usize, series: Series, orient: T)
    where
        T: Into<DataOrientation>,
    {
        let mut series = series;
        let orient: DataOrientation = orient.into();

        match orient {
            DataOrientation::Horizontal => {
                let mut processor = DataframeRowProcessor::new(RefCols::D);

                for i in 0..self.size.1 + 1 {
                    match series.get_mut(i) {
                        Some(v) => processor.exec(i, v),
                        None => processor.skip(),
                    }
                }

                self.columns.insert(index, processor.get_cache_col());
                self.size.0 += 1;
                self.data.insert(index, processor.data);
            }
            DataOrientation::Vertical => {
                let mut processor = DataframeRowProcessor::new(RefCols::R(&self.columns));

                for i in 0..self.size.0 {
                    match series.get_mut(i) {
                        Some(v) => processor.exec(i, v),
                        None => processor.skip(),
                    }

                    self.data
                        .get_mut(i)
                        .unwrap()
                        .insert(index, processor.data.pop().unwrap());
                }

                self.insert_indices(index, orient);
            }
            DataOrientation::Raw => (),
        }
    }

    /// insert a series to a raw dataframe
    fn insert_r<T>(&mut self, index: usize, series: Series, orient: T)
    where
        T: Into<DataOrientation>,
    {
        let orient: DataOrientation = orient.into();

        match orient {
            DataOrientation::Horizontal => self.data.insert(index, series),
            DataOrientation::Vertical => {
                self.data
                    .iter_mut()
                    .zip(series.into_iter())
                    .for_each(|(v, i)| {
                        v.insert(index, i);
                    })
            }
            DataOrientation::Raw => (),
        }
    }

    /// insert data
    pub fn insert<T>(&mut self, index: usize, series: Series, orient: T)
    where
        T: Into<DataOrientation>,
    {
        if series.len() == 0 {
            return;
        }
        match self.data_orientation {
            DataOrientation::Horizontal => self.insert_h(index, series, orient),
            DataOrientation::Vertical => self.insert_v(index, series, orient),
            DataOrientation::Raw => self.insert_r(index, series, orient),
        }
    }

    /// batch insert
    pub fn insert_many<T>(&mut self, index: usize, dataframe: DF, orient: T)
    where
        T: Into<DataOrientation>,
    {
        let orient: DataOrientation = orient.into();

        for (i, v) in dataframe.into_iter().enumerate() {
            self.insert(i + index, v, orient.clone());
        }
    }

    /// truncate, clear all data but columns and data_orientation
    pub fn truncate(&mut self) {
        self.data = vec![];
        self.indices = vec![];
        self.size = (0, 0);
    }

    /// delete a series from a horizontal orientation dataframe
    fn delete_h<T>(&mut self, index: usize, orient: T)
    where
        T: Into<DataOrientation>,
    {
        let orient: DataOrientation = orient.into();

        match orient {
            DataOrientation::Horizontal => {
                if index > self.size.0 {
                    return;
                }
                self.data.remove(index);
                self.indices.remove(index);
                self.size.0 -= 1;
            }
            DataOrientation::Vertical => {
                if index > self.size.1 {
                    return;
                }
                self.data.iter_mut().for_each(|v| {
                    v.remove(index);
                });
                self.columns.remove(index);
                self.size.1 -= 1;
            }
            DataOrientation::Raw => (),
        }
    }

    /// delete a series from a vertical orientation dataframe
    fn delete_v<T>(&mut self, index: usize, orient: T)
    where
        T: Into<DataOrientation>,
    {
        let orient: DataOrientation = orient.into();

        match orient {
            DataOrientation::Horizontal => {
                if index > self.size.0 {
                    return;
                }
                self.data.remove(index);
                self.columns.remove(index);
                self.size.0 -= 1;
            }
            DataOrientation::Vertical => {
                if index > self.size.1 {
                    return;
                }
                self.data.iter_mut().for_each(|v| {
                    v.remove(index);
                });
                self.indices.remove(index);
                self.size.1 -= 1;
            }
            DataOrientation::Raw => (),
        }
    }

    /// delete a series from a raw dataframe
    fn delete_r<T>(&mut self, index: usize, orient: T)
    where
        T: Into<DataOrientation>,
    {
        let orient: DataOrientation = orient.into();

        match orient {
            DataOrientation::Horizontal => {
                self.data.remove(index);
            }
            DataOrientation::Vertical => {
                for v in self.data.iter_mut() {
                    v.remove(index);
                }
            }
            DataOrientation::Raw => (),
        }
    }

    /// delete a specific series, row-wise or column-wise
    pub fn delete<T>(&mut self, index: usize, orient: T)
    where
        T: Into<DataOrientation>,
    {
        let orient: DataOrientation = orient.into();

        match orient {
            DataOrientation::Horizontal => self.delete_h(index, orient),
            DataOrientation::Vertical => self.delete_v(index, orient),
            DataOrientation::Raw => self.delete_r(index, orient),
        }
    }

    /// batch delete
    pub fn delete_many<T>(&mut self, indices: &[usize], orient: T)
    where
        T: Into<DataOrientation>,
    {
        let orient: DataOrientation = orient.into();
        let mut indices = indices.to_vec();
        indices.sort_by(|a, b| b.cmp(a));

        for i in indices {
            self.delete(i, orient.clone());
        }
    }
}

/// Convert dataframe to pure DF structure
impl From<Dataframe> for DF {
    fn from(dataframe: Dataframe) -> Self {
        match &dataframe.data_orientation {
            DataOrientation::Horizontal => {
                let mut dataframe = dataframe;
                let head = dataframe
                    .columns
                    .into_iter()
                    .map(|d| d.name.into())
                    .collect::<Vec<_>>();
                dataframe.data.insert(0, head);
                dataframe.data
            }
            DataOrientation::Vertical => dataframe
                .data
                .into_iter()
                .zip(dataframe.columns.into_iter())
                .map(|(mut row, cd)| {
                    row.insert(0, cd.name.into());
                    row
                })
                .collect::<Vec<_>>(),
            DataOrientation::Raw => dataframe.data,
        }
    }
}

/// iterator returns `Series` (takes ownership)
impl IntoIterator for Dataframe {
    type Item = Series;
    type IntoIter = IntoIteratorDf;

    fn into_iter(self) -> Self::IntoIter {
        IntoIteratorDf {
            iter: self.data.into_iter(),
        }
    }
}

pub struct IntoIteratorDf {
    iter: std::vec::IntoIter<Series>,
}

impl Iterator for IntoIteratorDf {
    type Item = Series;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

/// iterator returns `&Series`
impl<'a> IntoIterator for &'a Dataframe {
    type Item = &'a Series;
    type IntoIter = IteratorDf<'a>;

    fn into_iter(self) -> Self::IntoIter {
        IteratorDf {
            iter: self.data.iter(),
        }
    }
}

pub struct IteratorDf<'a> {
    iter: std::slice::Iter<'a, Series>,
}

impl<'a> Iterator for IteratorDf<'a> {
    type Item = &'a Series;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

/// iterator returns `&mut Series`
impl<'a> IntoIterator for &'a mut Dataframe {
    type Item = &'a mut Series;
    type IntoIter = IterMutDf<'a>;

    fn into_iter(self) -> Self::IntoIter {
        IterMutDf {
            iter: self.data.iter_mut(),
        }
    }
}

pub struct IterMutDf<'a> {
    iter: std::slice::IterMut<'a, Series>,
}

impl<'a> Iterator for IterMutDf<'a> {
    type Item = &'a mut Series;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

/// impl `iter` & `iter_mut` methods for `Dataframe`
impl<'a> Dataframe {
    pub fn iter(&'a self) -> IteratorDf<'a> {
        self.into_iter()
    }

    pub fn iter_mut(&'a mut self) -> IterMutDf<'a> {
        self.into_iter()
    }
}

#[cfg(test)]
mod tiny_df_test {
    use chrono::NaiveDate;

    use crate::{df, series};

    use super::*;

    const DIVIDER: &'static str = "-------------------------------------------------------------";

    #[test]
    fn test_df_new_h() {
        let data: DF = df![
            ["date", "object", "value"],
            [NaiveDate::from_ymd(2000, 1, 1), "A", 5],
        ];
        let df = Dataframe::new(data, "h");
        println!("{:#?}", df);
        println!("{:?}", DIVIDER);

        let data: DF = df![
            ["date", "object"],
            [NaiveDate::from_ymd(2000, 1, 1), "A", 5],
            [NaiveDate::from_ymd(2010, 6, 1), "B", 23, "out of bound",],
            [NaiveDate::from_ymd(2020, 10, 1), 22, 38,],
        ];
        let df = Dataframe::new(data, "h");
        println!("{:#?}", df);
        println!("{:?}", DIVIDER);
    }

    #[test]
    fn test_df_new_v() {
        let data: DF = df![
            [
                "date",
                NaiveDate::from_ymd(2000, 1, 1),
                NaiveDate::from_ymd(2010, 6, 1),
                NaiveDate::from_ymd(2020, 10, 1),
            ],
            ["object", "A", "B", "C"],
            ["value", 5, "wrong num", 23],
        ];
        let df = Dataframe::new(data, "V");
        println!("{:#?}", df);
        println!("{:?}", DIVIDER);

        let data: DF = df![
            [
                "date",
                NaiveDate::from_ymd(2000, 1, 1),
                NaiveDate::from_ymd(2010, 6, 1),
            ],
            ["object", "A", "B", "C"],
            ["value", 5, 23],
        ];
        let df = Dataframe::new(data, "v");
        println!("{:#?}", df);
        println!("{:?}", DIVIDER);

        let data: DF = df![["date",], ["object",], ["value",],];
        let df = Dataframe::new(data, "v");
        println!("{:#?}", df);
        println!("{:?}", DIVIDER);
    }

    #[test]
    fn test_df_new_h_col() {
        let data: DF = df![
            [NaiveDate::from_ymd(2000, 1, 1), "A", 5],
            [NaiveDate::from_ymd(2010, 6, 1), "B", 23, "out of bound",],
            [NaiveDate::from_ymd(2020, 10, 1), 22, 38,],
            [NaiveDate::from_ymd(2030, 5, 1), DataframeData::None, 3,],
        ];
        let col = vec![
            DataframeColumn::new("date", DataType::Date),
            DataframeColumn::new("object", DataType::String),
            DataframeColumn::new("value", DataType::Short),
        ];
        let df = Dataframe::from_2d_vec(data, "h", col);
        println!("{:#?}", df);
        println!("{:?}", DIVIDER);
    }

    #[test]
    fn test_df_new_v_col() {
        let data: DF = df![
            [
                NaiveDate::from_ymd(2000, 1, 1),
                NaiveDate::from_ymd(2010, 6, 1),
                NaiveDate::from_ymd(2020, 10, 1),
                NaiveDate::from_ymd(2030, 10, 1),
            ],
            ["A", "B", "C"],
            [5, "wrong num", 23],
        ];
        let col = vec![
            DataframeColumn::new("date", DataType::Date),
            DataframeColumn::new("object", DataType::String),
            DataframeColumn::new("value", DataType::Short),
        ];
        let df = Dataframe::from_2d_vec(data, "v", col);
        println!("{:#?}", df);
        println!("{:?}", DIVIDER);
    }

    #[test]
    fn test_df_transpose() {
        let data: DF = df![
            [
                "date",
                NaiveDate::from_ymd(2000, 1, 1),
                NaiveDate::from_ymd(2010, 6, 1),
                NaiveDate::from_ymd(2020, 10, 1),
                NaiveDate::from_ymd(2030, 1, 1),
            ],
            ["object", "A", "B", "C", "D",],
            ["value", 5, "wrong num", 23, 0,],
        ];
        let mut df = Dataframe::new(data, "V");
        println!("{:#?}", df);
        println!("{:?}", DIVIDER);

        df.transpose();
        println!("{:#?}", df);
    }

    #[test]
    fn test_df_h_append() {
        let data = df![
            ["date", "object", "value"],
            [NaiveDate::from_ymd(2000, 1, 1), "A", 5],
            [NaiveDate::from_ymd(2010, 6, 1), "B", 23, "out of bound",],
            [NaiveDate::from_ymd(2020, 10, 1), 22, 38,],
        ];
        let mut df = Dataframe::new(data, "H");
        let extra = series![
            NaiveDate::from_ymd(2030, 1, 1),
            "K",
            "wrong type",
            "out of bound",
        ];

        df.append(extra);

        println!("{:#?}", df);
        println!("{:?}", DIVIDER);
    }

    #[test]
    fn test_df_v_append() {
        let data: DF = df![
            [
                "date",
                NaiveDate::from_ymd(2000, 1, 1),
                NaiveDate::from_ymd(2010, 6, 1),
                NaiveDate::from_ymd(2020, 10, 1),
            ],
            ["object", "A", "B", "C"],
            ["value", 5, "wrong num", 23],
        ];
        let mut df = Dataframe::new(data, "v");
        let extra = series!["Note", "K", "B", "A",];

        df.append(extra);

        println!("{:#?}", df);
        println!("{:?}", DIVIDER);
    }

    #[test]
    fn test_df_h_concat() {
        let data = df![
            ["date", "object", "value"],
            [NaiveDate::from_ymd(2000, 1, 1), "A", 5],
            [NaiveDate::from_ymd(2010, 6, 1), "B", 23, "out of bound",],
            [NaiveDate::from_ymd(2020, 10, 1), 22, 38,],
        ];
        let mut df = Dataframe::new(data, "H");
        let extra = df![
            [
                NaiveDate::from_ymd(2030, 1, 1),
                "K",
                "wrong type",
                "out of bound",
            ],
            [NaiveDate::from_ymd(2040, 3, 1), "Q", 18, "out of bound",]
        ];

        df.concat(extra);

        println!("{:#?}", df);
        println!("{:?}", DIVIDER);
    }

    #[test]
    fn test_df_v_concat() {
        let data: DF = df![
            [
                "date",
                NaiveDate::from_ymd(2000, 1, 1),
                NaiveDate::from_ymd(2010, 6, 1),
                NaiveDate::from_ymd(2020, 10, 1),
            ],
            ["object", "A", "B", "C"],
            ["value", 5, "wrong num", 23],
        ];
        let mut df = Dataframe::new(data, "v");
        let extra = df![["Note", "K", "B", "A",], ["PS", 1, "worong type", 2,],];

        df.concat(extra);

        println!("{:#?}", df);
        println!("{:?}", DIVIDER);
    }

    #[test]
    fn test_df_col_rename() {
        let data = df![
            ["idx", "name", "tag"],
            [0, "Jacob", "Cool"],
            [1, "Sam", "Mellow"],
        ];

        let mut df = Dataframe::new(data, "h");

        df.rename_column(2, "kind");
        println!("{:#?}", df.columns());

        df.rename_column(5, "OoB");
        println!("{:#?}", df.columns());
    }

    #[test]
    fn test_df_col_renames() {
        let data = df![
            ["idx", "name", "tag"],
            [0, "Jacob", "Cool"],
            [1, "Sam", "Mellow"],
        ];

        let mut df = Dataframe::new(data, "h");

        df.rename_columns(&["index", "nickname"]);
        println!("{:#?}", df.columns());

        df.rename_columns(&["index", "nickname", "tag", "OoB"]);
        println!("{:#?}", df.columns());
    }

    #[test]
    fn test_df_index_replace() {
        let data = df![
            ["idx", "name", "tag"],
            [0, "Jacob", "Cool"],
            [1, "Sam", "Mellow"],
        ];

        let mut df = Dataframe::new(data, "h");

        df.replace_index(1, "233");
        println!("{:#?}", df.indices());
    }

    #[test]
    fn test_df_indices_replace() {
        let data = df![
            ["idx", "name", "tag"],
            [0, "Jacob", "Cool"],
            [1, "Sam", "Mellow"],
        ];

        let mut df = Dataframe::new(data, "h");

        df.replace_indices(&["one"]);
        println!("{:#?}", df.indices());

        df.replace_indices(&["壹", "贰", "叁", "肆"]);
        println!("{:#?}", df.indices());
    }

    #[test]
    fn test_df_truncate() {
        let data = df![
            ["idx", "name", "tag"],
            [0, "Jacob", "Cool"],
            [1, "Sam", "Mellow"],
        ];

        let mut df = Dataframe::new(data, "h");

        println!("{:#?}", df);

        df.truncate();
        println!("{:#?}", df);
    }

    #[test]
    fn test_df_iter() {
        let data = df![
            ["idx", "name", "tag"],
            [0, "Jacob", "Cool"],
            [1, "Sam", "Mellow"],
        ];

        let mut df = Dataframe::new(data, "h");

        df.iter().for_each(|i| {
            println!("{:?}", i);
        });

        // mutate `df`, mocking insert index to each row
        df.iter_mut()
            .enumerate()
            .for_each(|(idx, v)| v.insert(0, DataframeData::Id(idx as u64)));

        println!("{:#?}", df);
    }

    #[test]
    fn test_df_h_insert_h() {
        let data = df![
            ["idx", "name", "tag"],
            [0, "Jacob", "Cool"],
            [1, "Sam", "Mellow"],
        ];

        let mut df = Dataframe::new(data, "h");

        let s = series![2, "Box", "Pure"];

        df.insert(1, s, "h");

        println!("{:#?}", df);
    }

    #[test]
    fn test_df_h_insert_v() {
        let data = df![
            ["idx", "name", "tag"],
            [0, "Jacob", "Cool"],
            [1, "Sam", "Mellow"],
        ];

        let mut df = Dataframe::new(data, "h");

        let s = series!["note", "#1", "#2"];

        df.insert(2, s, "v");

        println!("{:#?}", df);
    }

    #[test]
    fn test_df_v_insert_h() {
        let data = df![
            ["idx", 0, 1, 2],
            ["name", "Jacob", "Sam", "Mia"],
            ["tag", "Cool", "Mellow", "Enthusiastic"],
        ];

        let mut df = Dataframe::new(data, "v");

        let s = series!["note", "#1", "#2"];

        df.insert(1, s, "h");

        println!("{:#?}", df);
    }

    #[test]
    fn test_df_v_insert_v() {
        let data = df![
            ["idx", 0, 1],
            ["name", "Jacob", "Sam"],
            ["tag", "Cool", "Mellow"],
        ];

        let mut df = Dataframe::new(data, "V");

        let s = series![2, "Box", "Pure", "OoB"];

        df.insert(2, s, "V");

        println!("{:#?}", df);
    }

    #[test]
    fn test_df_h_delete_h() {
        let data = df![
            ["idx", "name", "tag"],
            [0, "Jacob", "Cool"],
            [1, "Sam", "Mellow"],
            [2, "Mia", "Soft"],
        ];

        let mut df = Dataframe::new(data, "h");

        df.delete(1, "h");

        println!("{:#?}", df);
    }

    #[test]
    fn test_df_h_delete_v() {
        let data = df![
            ["idx", "name", "tag"],
            [0, "Jacob", "Cool"],
            [1, "Sam", "Mellow"],
            [2, "Mia", "Soft"],
        ];

        let mut df = Dataframe::new(data, "h");

        df.delete(1, "v");

        println!("{:#?}", df);
    }

    #[test]
    fn test_df_v_delete_h() {
        let data = df![
            ["idx", 0, 1, 2],
            ["name", "Jacob", "Sam", "Mia"],
            ["tag", "Cool", "Mellow", "Enthusiastic"],
        ];

        let mut df = Dataframe::new(data, "v");

        df.delete(1, "h");

        println!("{:#?}", df);
    }

    #[test]
    fn test_df_v_delete_v() {
        let data = df![
            ["idx", 0, 1, 2],
            ["name", "Jacob", "Sam", "Mia"],
            ["tag", "Cool", "Mellow", "Enthusiastic"],
        ];

        let mut df = Dataframe::new(data, "V");

        df.delete(2, "V");

        println!("{:#?}", df);
    }

    #[test]
    fn test_df_h_iloc_loc() {
        let data = df![
            ["idx", "name", "tag"],
            [0, "Jacob", "Cool"],
            [1, "Sam", "Mellow"],
            [2, "Mia", "Soft"],
        ];

        let mut df = Dataframe::new(data, "h");

        println!("{:?}", df.iloc(1, 2));

        df.replace_indices(&["壹", "贰", "叁"]);

        println!("{:?}", df.loc("叁", "name"));
    }

    #[test]
    fn test_df_v_iloc_loc() {
        let data = df![
            ["idx", 0, 1, 2],
            ["name", "Jacob", "Sam", "Mia"],
            ["tag", "Cool", "Mellow", "Enthusiastic"],
        ];

        let mut df = Dataframe::new(data, "v");

        println!("{:?}", df.iloc(1, 0));

        df.replace_indices(&["壹", "贰", "叁"]);

        println!("{:?}", df.loc("壹", "tag"));
    }
}
