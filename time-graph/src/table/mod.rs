#![allow(clippy::needless_range_loop)]

//! This module contains a copy of https://github.com/RyanBluth/term-table-rs at
//! commit ed20913, simplified for usage inside time-graph.
//!
//! The code was copied to remove a dependency on regex, cutting down the
//! compile time.

mod row;
mod table_cell;

pub use self::row::Row;
pub use self::table_cell::TableCell;
use self::table_cell::Alignment;

use std::cmp::{max, min};
use std::collections::HashMap;

/// Represents the vertical position of a row
#[derive(Eq, PartialEq, Copy, Clone)]
pub enum RowPosition {
    First,
    Mid,
    Last,
}

/// A set of characters which make up a table style
///
///# Example
///
///```
/// term_table::TableStyle {
///     top_left_corner: '╔',
///     top_right_corner: '╗',
///     bottom_left_corner: '╚',
///     bottom_right_corner: '╝',
///     outer_left_vertical: '╠',
///     outer_right_vertical: '╣',
///     outer_bottom_horizontal: '╩',
///     outer_top_horizontal: '╦',
///     intersection: '╬',
///     vertical: '║',
///     horizontal: '═',
/// };
/// ```
#[derive(Debug, Clone, Copy)]
pub struct TableStyle {
    pub top_left_corner: char,
    pub top_right_corner: char,
    pub bottom_left_corner: char,
    pub bottom_right_corner: char,
    pub outer_left_vertical: char,
    pub outer_right_vertical: char,
    pub outer_bottom_horizontal: char,
    pub outer_top_horizontal: char,
    pub intersection: char,
    pub vertical: char,
    pub horizontal: char,
}

impl TableStyle {
    /// Table style using extended character set
    ///
    ///# Example
    ///
    ///<pre>
    /// ╔═════════════════════════════════════════════════════════════════════════════════╗
    /// ║                            This is some centered text                           ║
    /// ╠════════════════════════════════════════╦════════════════════════════════════════╣
    /// ║ This is left aligned text              ║             This is right aligned text ║
    /// ╠════════════════════════════════════════╬════════════════════════════════════════╣
    /// ║ This is left aligned text              ║             This is right aligned text ║
    /// ╠════════════════════════════════════════╩════════════════════════════════════════╣
    /// ║ This is some really really really really really really really really really tha ║
    /// ║ t is going to wrap to the next line                                             ║
    /// ╚═════════════════════════════════════════════════════════════════════════════════╝
    ///</pre>
    pub fn extended() -> TableStyle {
        TableStyle {
            top_left_corner: '╔',
            top_right_corner: '╗',
            bottom_left_corner: '╚',
            bottom_right_corner: '╝',
            outer_left_vertical: '╠',
            outer_right_vertical: '╣',
            outer_bottom_horizontal: '╩',
            outer_top_horizontal: '╦',
            intersection: '╬',
            vertical: '║',
            horizontal: '═',
        }
    }

    /// Returns the start character of a table style based on the
    /// vertical position of the row
    fn start_for_position(&self, pos: RowPosition) -> char {
        match pos {
            RowPosition::First => self.top_left_corner,
            RowPosition::Mid => self.outer_left_vertical,
            RowPosition::Last => self.bottom_left_corner,
        }
    }

    /// Returns the end character of a table style based on the
    /// vertical position of the row
    fn end_for_position(&self, pos: RowPosition) -> char {
        match pos {
            RowPosition::First => self.top_right_corner,
            RowPosition::Mid => self.outer_right_vertical,
            RowPosition::Last => self.bottom_right_corner,
        }
    }

    /// Returns the intersect character of a table style based on the
    /// vertical position of the row
    fn intersect_for_position(&self, pos: RowPosition) -> char {
        match pos {
            RowPosition::First => self.outer_top_horizontal,
            RowPosition::Mid => self.intersection,
            RowPosition::Last => self.outer_bottom_horizontal,
        }
    }

    /// Merges two intersecting characters based on the vertical position of a row.
    /// This is used to handle cases where one cell has a larger `col_span` value than the other
    fn merge_intersection_for_position(&self, top: char, bottom: char, pos: RowPosition) -> char {
        if (top == self.horizontal || top == self.outer_bottom_horizontal)
            && bottom == self.intersection
        {
            return self.outer_top_horizontal;
        } else if (top == self.intersection || top == self.outer_top_horizontal)
            && bottom == self.horizontal
        {
            return self.outer_bottom_horizontal;
        } else if top == self.outer_bottom_horizontal && bottom == self.horizontal {
            return self.horizontal;
        } else {
            return self.intersect_for_position(pos);
        }
    }
}

/// A set of rows containing data
#[derive(Clone, Debug)]
pub struct Table {
    pub rows: Vec<Row>,
    pub style: TableStyle,
    /// The maximum width of all columns. Overridden by values in column_widths. Defaults to `std::usize::max`
    pub max_column_width: usize,
    /// The maximum widths of specific columns. Override max_column
    pub max_column_widths: HashMap<usize, usize>,
    /// Whether or not to vertically separate rows in the table
    pub separate_rows: bool,
    /// Whether the table should have a top boarder.
    /// Setting `has_separator` to false on the first row will have the same effect as setting this to false
    pub has_top_boarder: bool,
    /// Whether the table should have a bottom boarder
    pub has_bottom_boarder: bool,
}

impl Table {
    pub fn new() -> Table {
        Self {
            rows: Vec::new(),
            style: TableStyle::extended(),
            max_column_width: usize::MAX,
            max_column_widths: HashMap::new(),
            separate_rows: true,
            has_top_boarder: true,
            has_bottom_boarder: true,
        }
    }

    /// Simply adds a row to the rows Vec
    pub fn add_row(&mut self, row: Row) {
        self.rows.push(row);
    }

    /// Does all of the calculations to reformat the row based on it's current
    /// state and returns the result as a `String`
    pub fn render(&self) -> String {
        let mut print_buffer = String::new();
        let max_widths = self.calculate_max_column_widths();
        let mut previous_separator = None;
        if !self.rows.is_empty() {
            for i in 0..self.rows.len() {
                let row_pos = if i == 0 {
                    RowPosition::First
                } else {
                    RowPosition::Mid
                };

                let separator = self.rows[i].gen_separator(
                    &max_widths,
                    &self.style,
                    row_pos,
                    previous_separator.clone(),
                );

                previous_separator = Some(separator.clone());

                if self.rows[i].has_separator
                    && ((i == 0 && self.has_top_boarder) || i != 0 && self.separate_rows)
                {
                    Table::buffer_line(&mut print_buffer, &separator);
                }

                Table::buffer_line(
                    &mut print_buffer,
                    &self.rows[i].format(&max_widths, &self.style),
                );
            }
            if self.has_bottom_boarder {
                let separator = self.rows.last().unwrap().gen_separator(
                    &max_widths,
                    &self.style,
                    RowPosition::Last,
                    None,
                );
                Table::buffer_line(&mut print_buffer, &separator);
            }
        }
        return print_buffer;
    }

    /// Calculates the maximum width for each column.
    /// If a cell has a column span greater than 1, then the width
    /// of it's contents are divided by the column span, otherwise the cell
    /// would use more space than it needed.
    fn calculate_max_column_widths(&self) -> Vec<usize> {
        let mut num_columns = 0;

        for row in &self.rows {
            num_columns = max(row.num_columns(), num_columns);
        }
        let mut max_widths: Vec<usize> = vec![0; num_columns];
        let mut min_widths: Vec<usize> = vec![0; num_columns];
        for row in &self.rows {
            let column_widths = row.split_column_widths();
            for i in 0..column_widths.len() {
                min_widths[i] = max(min_widths[i], column_widths[i].1);
                let mut max_width = *self
                    .max_column_widths
                    .get(&i)
                    .unwrap_or(&self.max_column_width);
                max_width = max(min_widths[i], max_width);
                max_widths[i] = min(max_width, max(max_widths[i], column_widths[i].0 as usize));
            }
        }

        // Here we are dealing with the case where we have a cell that is center
        // aligned but the max_width doesn't allow for even padding on either side
        for row in &self.rows {
            let mut col_index = 0;
            for cell in row.cells.iter() {
                let mut total_col_width = 0;
                for i in col_index..col_index + cell.col_span {
                    total_col_width += max_widths[i];
                }
                if cell.width() != total_col_width
                    && cell.alignment == Alignment::Center
                    && total_col_width as f32 % 2.0 <= 0.001
                {
                    let mut max_col_width = self.max_column_width;
                    if let Some(specific_width) = self.max_column_widths.get(&col_index) {
                        max_col_width = *specific_width;
                    }

                    if max_widths[col_index] < max_col_width {
                        max_widths[col_index] += 1;
                    }
                }
                if cell.col_span > 1 {
                    col_index += cell.col_span - 1;
                } else {
                    col_index += 1;
                }
            }
        }

        return max_widths;
    }

    /// Helper method for adding a line to a string buffer
    fn buffer_line(buffer: &mut String, line: &str) {
        buffer.push_str(format!("{}\n", line).as_str());
    }
}
