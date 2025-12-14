/// Licensed to the Apache Software Foundation (ASF) under one or more
/// contributor license agreements. See the NOTICE file distributed with
/// this work for additional information regarding copyright ownership.
/// The ASF licenses this file to You under the Apache License, Version 2.0
/// (the "License"); you may not use this file except in compliance with
/// the License. You may obtain a copy of the License at
///
/// http://www.apache.org/licenses/LICENSE-2.0
///
/// Unless required by applicable law or agreed to in writing, software
/// distributed under the License is distributed on an "AS IS" BASIS,
/// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
/// See the License for the specific language governing permissions and
/// limitations under the License.
use std::fmt::Debug;

/// Represents an Excel hyperlink.
pub trait Hyperlink: Debug + crate::core::poi::common::usermodel::hyperlink::Hyperlink {
    /// Return the row of the first cell that contains the hyperlink.
    ///
    /// # Returns
    /// The 0-based row of the cell that contains the hyperlink.
    fn get_first_row(&self) -> u32;

    /// Set the row of the first cell that contains the hyperlink.
    ///
    /// # Arguments
    /// * `row` - The 0-based row of the first cell that contains the hyperlink.
    fn set_first_row(&mut self, row: u32);

    /// Return the row of the last cell that contains the hyperlink.
    ///
    /// # Returns
    /// The 0-based row of the last cell that contains the hyperlink.
    fn get_last_row(&self) -> u32;

    /// Set the row of the last cell that contains the hyperlink.
    ///
    /// # Arguments
    /// * `row` - The 0-based row of the last cell that contains the hyperlink.
    fn set_last_row(&mut self, row: u32);

    /// Return the column of the first cell that contains the hyperlink.
    ///
    /// # Returns
    /// The 0-based column of the first cell that contains the hyperlink.
    fn get_first_column(&self) -> u32;

    /// Set the column of the first cell that contains the hyperlink.
    ///
    /// # Arguments
    /// * `col` - The 0-based column of the first cell that contains the hyperlink.
    fn set_first_column(&mut self, col: u32);

    /// Return the column of the last cell that contains the hyperlink.
    ///
    /// # Returns
    /// The 0-based column of the last cell that contains the hyperlink.
    fn get_last_column(&self) -> u32;

    /// Set the column of the last cell that contains the hyperlink.
    ///
    /// # Arguments
    /// * `col` - The 0-based column of the last cell that contains the hyperlink.
    fn set_last_column(&mut self, col: u32);
}
