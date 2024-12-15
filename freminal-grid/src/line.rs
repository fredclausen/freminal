// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::borrow::Cow;

/// The cell trait
/// A cell is a single unit of a line.
/// The default for a cell object is to be empty and not displayed.
pub trait Cell: Sized + Default + Clone + Eq + PartialEq {
    /// Check if the cell is empty.
    fn is_empty(&self) -> bool;
    fn reset(&mut self);
    fn empty_self() -> Self;
}

pub struct Line<T> {
    inner: Vec<T>,
    max_length: usize,
}

impl<T: Cell> Line<T> {
    #[must_use]
    pub fn new(length: usize) -> Self {
        Self {
            inner: Vec::with_capacity(length),
            max_length: length,
        }
    }

    #[must_use]
    pub fn get_visible_cells(&self) -> Vec<&T> {
        self.inner.iter().filter(|cell| !cell.is_empty()).collect()
    }

    #[must_use]
    pub fn get_all_cells(&self) -> Vec<&T> {
        self.inner.iter().collect()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.iter().all(Cell::is_empty)
    }

    pub fn reset(&mut self) {
        for cell in &mut self.inner {
            cell.reset();
        }
    }

    /// Insert a single item at the given index, provided the index is within the bounds of the line.
    /// If the insert is successful, None is returned. If the index is out of bounds, the item is returned.
    pub fn insert_at_single_item(&mut self, index: usize, item: T) -> Option<T> {
        // insert at the given index. If the index is out of bounds, insert the default value, up to the index we're inserting to.
        // and then insert the item.
        // If the index, or insertion would cause the max length to be exceeded, return the item.
        if index >= self.max_length {
            trace!("Index out of bounds: {}", index);
            return Some(item);
        }

        if index + 1 > self.max_length {
            trace!("Insertion would exceed max length: {}", index);
            return Some(item);
        }

        if index >= self.inner.len() {
            self.inner.resize(index, T::default());
            self.inner.push(item);
        } else {
            self.inner.insert(index, item);
        }

        None
    }

    pub fn insert_multiple_items(&mut self, index: usize, items: Vec<T>) -> Option<Vec<T>> {
        if index >= self.max_length {
            trace!("Index out of bounds: {}", index);
            return Some(items);
        }

        if index + items.len() > self.max_length {
            trace!("Insertion would exceed max length: {}", index);
            return Some(items);
        }

        if index >= self.inner.len() {
            self.inner.resize(index, T::default());
            self.inner.extend(items);
        } else {
            self.inner.splice(index..index, items);
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[derive(Default, Clone, Eq, PartialEq, Debug)]
    struct TestCell {
        value: u8,
    }

    impl Cell for TestCell {
        fn is_empty(&self) -> bool {
            self.value == u8::MAX
        }

        fn reset(&mut self) {
            self.value = 0;
        }

        fn empty_self() -> Self {
            Self { value: u8::MAX }
        }
    }

    #[test]
    fn test_new() {
        let line = Line::<TestCell>::new(10);
        assert_eq!(line.inner.len(), 0);
        assert_eq!(line.max_length, 10);
    }

    #[test]
    fn test_insert_vector_of_items() {
        let mut line = Line::<TestCell>::new(10);
        let item = TestCell { value: 1 };
        let item2 = TestCell { value: 2 };
        let default_item = TestCell::default();

        assert_eq!(
            line.insert_multiple_items(0, vec![item.clone(), item2.clone()]),
            None
        );
        assert_eq!(
            line.get_visible_cells(),
            vec![&item, &item2, &default_item, &default_item, &default_item]
        );
        assert_eq!(
            line.insert_multiple_items(3, vec![item.clone(), item2.clone()]),
            None
        );
        assert_eq!(
            line.get_visible_cells(),
            vec![
                &item,
                &item2,
                &default_item,
                &item,
                &item2,
                &default_item,
                &default_item,
                &default_item,
                &default_item,
                &default_item
            ]
        );
        assert_eq!(
            line.insert_multiple_items(10, vec![item.clone(), item2.clone()]),
            Some(vec![item.clone(), item2.clone()])
        );
        assert_eq!(
            line.get_visible_cells(),
            vec![
                &item,
                &item2,
                &default_item,
                &item,
                &item2,
                &default_item,
                &default_item,
                &default_item,
                &default_item,
                &item
            ]
        );
    }

    #[test]
    fn test_insert_at_single_item() {
        let mut line = Line::<TestCell>::new(10);
        let item = TestCell { value: 1 };
        let item2 = TestCell { value: 2 };
        let default_item = TestCell::default();

        assert_eq!(line.insert_at_single_item(0, item.clone()), None);
        assert_eq!(line.insert_at_single_item(1, item2.clone()), None);
        assert_eq!(
            line.insert_at_single_item(10, item.clone()),
            Some(item.clone())
        );
        assert_eq!(line.get_visible_cells(), vec![&item, &item2]);
        assert_eq!(line.insert_at_single_item(9, item2.clone()), None);
        assert_eq!(
            line.get_visible_cells(),
            vec![
                &item,
                &item2,
                &default_item,
                &default_item,
                &default_item,
                &default_item,
                &default_item,
                &default_item,
                &default_item,
                &item2
            ]
        );
    }
}
