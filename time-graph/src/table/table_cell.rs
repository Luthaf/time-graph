use std::cmp;

use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

/// Represents the horizontal alignment of content within a cell.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Alignment {
    Left,
    Right,
    Center,
}

/// A table cell containing some str data.
///
/// A cell may span multiple columns by setting the value of `col_span`.
///
/// `pad_content` will add a space to either side of the cell's content.
#[derive(Debug, Clone)]
pub struct TableCell {
    pub data: String,
    pub col_span: usize,
    pub alignment: Alignment,
    pub pad_content: bool,
}

impl TableCell {
    pub fn new<T>(data: T) -> TableCell
    where
        T: ToString,
    {
        Self {
            data: data.to_string(),
            col_span: 1,
            alignment: Alignment::Left,
            pad_content: true,
        }
    }

    pub fn new_right_aligned<T>(data: T) -> TableCell
    where
        T: ToString,
    {
        Self {
            data: data.to_string(),
            pad_content: true,
            col_span: 1,
            alignment: Alignment::Right,
        }
    }

    /// Calculates the width of the cell.
    ///
    /// New line characters are taken into account during the calculation.
    pub fn width(&self) -> usize {
        let wrapped = self.wrapped_content(usize::MAX);
        let mut max = 0;
        for s in wrapped {
            let str_width = s.width();
            max = cmp::max(max, str_width);
        }
        max
    }

    /// The width of the cell's content divided by its `col_span` value.
    pub fn split_width(&self) -> f32 {
        self.width() as f32 / self.col_span as f32
    }

    /// The minium width required to display the cell properly
    pub fn min_width(&self) -> usize {
        let mut max_char_width: usize = 0;
        for c in self.data.chars() {
            max_char_width = cmp::max(max_char_width, c.width().unwrap_or(1));
        }

        if self.pad_content {
            max_char_width + ' '.width().unwrap_or(1) * 2
        } else {
            max_char_width
        }
    }

    /// Wraps the cell's content to the provided width.
    ///
    /// New line characters are taken into account.
    pub fn wrapped_content(&self, width: usize) -> Vec<String> {
        let pad_char = if self.pad_content { ' ' } else { '\0' };

        let mut res: Vec<String> = Vec::new();
        let mut buf = String::new();
        buf.push(pad_char);
        for c in self.data.chars() {
            if buf.width() >= width - pad_char.width().unwrap_or(1) || c == '\n' {
                buf.push(pad_char);
                res.push(buf);
                buf = String::new();
                buf.push(pad_char);
                if c == '\n' {
                    continue;
                }
            }
            buf.push(c);
        }
        buf.push(pad_char);
        res.push(buf);

        res
    }
}

impl<T> From<T> for TableCell
where
    T: ToString,
{
    fn from(other: T) -> Self {
        TableCell::new(other)
    }
}
