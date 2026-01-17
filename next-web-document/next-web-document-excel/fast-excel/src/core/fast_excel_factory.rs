use std::path::Path;

use crate::core::read::builder::base_excel_reader_parameter_builder::BaseExcelReaderParameterBuilder;
use crate::core::write::builder::excel_writer_sheet_builder::ExcelWriterSheetBuilder;
use crate::core::write::builder::excel_writer_table_builder::ExcelWriterTableBuilder;

use crate::core::{
    error::excel_error::ExcelError,
    read::{
        builder::{
            excel_reader_builder::ExcelReaderBuilder,
            excel_reader_sheet_builder::ExcelReaderSheetBuilder,
        },
        listener::read_listener::ReadListener,
    },
    write::builder::excel_writer_builder::ExcelWriterBuilder,
};

#[cfg(not(feature = "async"))]
use std::fs::File;
#[cfg(not(feature = "async"))]
use std::io::{Read, Write};
#[cfg(feature = "async")]
use tokio::fs::File;
#[cfg(feature = "async")]
use tokio::io::{AsyncRead as Read, AsyncWrite as Write};

#[cfg(not(feature = "async"))]
pub trait FastExcelFactory {
    fn write() {}
    fn write_with_file(file: StdFile) {}
    fn read() {}
}

#[cfg(feature = "async")]
pub trait FastExcelFactory {
    fn write() -> ExcelWriterBuilder<()> {
        ExcelWriterBuilder::default()
    }

    fn write_with_file(file: File) -> ExcelWriterBuilder<()> {
        Self::write().file(file)
    }

    fn write_with_head<T>(file: File) -> ExcelWriterBuilder<T> {
        ExcelWriterBuilder::default().file(file)
    }

    async fn write_with_path<P>(path: P) -> ExcelWriterBuilder<()>
    where
        P: AsRef<Path>,
    {
        ExcelWriterBuilder::default().file_with_path(path).await
    }

    async fn write_with_path_and_head<P, T>(path: P) -> ExcelWriterBuilder<T>
    where
        P: AsRef<Path>,
    {
        ExcelWriterBuilder::default().file_with_path(path).await
    }

    fn write_with_w<W>(writer: W) -> ExcelWriterBuilder<()>
    where
        W: Write + 'static,
    {
        ExcelWriterBuilder::default().writer(writer)
    }

    fn write_with_w_and_head<W, T>(writer: W) -> ExcelWriterBuilder<T>
    where
        W: Write + 'static,
    {
        ExcelWriterBuilder::default().writer(writer)
    }

    fn write_sheet() -> ExcelWriterSheetBuilder {
        ExcelWriterSheetBuilder::default()
    }

    fn write_sheet_with_no(sheet_no: u32) -> ExcelWriterSheetBuilder {
        ExcelWriterSheetBuilder::default().sheet_no(sheet_no)
    }

    fn write_sheet_with_name<S>(sheet_name: S) -> ExcelWriterSheetBuilder
    where
        S: ToString,
    {
        ExcelWriterSheetBuilder::default().sheet_name(sheet_name.to_string())
    }

    fn write_sheet_with_name_and_no<S>(sheet_no: u32, sheet_name: S) -> ExcelWriterSheetBuilder
    where
        S: ToString,
    {
        ExcelWriterSheetBuilder::default()
            .sheet_no(sheet_no)
            .sheet_name(sheet_name.to_string())
    }

    fn write_table() -> ExcelWriterTableBuilder {
        ExcelWriterTableBuilder::default()
    }

    fn write_table_with_no(table_no: u32) -> ExcelWriterTableBuilder {
        ExcelWriterTableBuilder::default().table_no(table_no)
    }

    fn read() -> ExcelReaderBuilder<()> {
        ExcelReaderBuilder::default()
    }

    fn read_with_file(file: File) -> ExcelReaderBuilder<()> {
        ExcelReaderBuilder::default().file(file)
    }

    fn read_with_listener<L>(file: File, read_listener: L) -> ExcelReaderBuilder<()>
    where
        L: ReadListener<()> + 'static,
    {
        let mut excel_reader_builder = ExcelReaderBuilder::default().file(file);
        excel_reader_builder.register_read_listener(read_listener);

        excel_reader_builder
    }

    fn read_with_head_and_listener<T>(
        file: File,
        read_listener: impl ReadListener<T> + 'static,
    ) -> ExcelReaderBuilder<T> {
        let mut excel_reader_builder = ExcelReaderBuilder::default().file(file);
        excel_reader_builder.register_read_listener(read_listener);

        excel_reader_builder
    }

    async fn read_with_path<P>(path: P) -> Result<ExcelReaderBuilder<()>, ExcelError>
    where
        P: AsRef<Path>,
    {
        let excel_reader_builder = ExcelReaderBuilder::default()
            .file_with_path_name(path)
            .await?;

        Ok(excel_reader_builder)
    }

    async fn read_with_path_and_listener<P, L>(
        path: P,
        read_listener: L,
    ) -> Result<ExcelReaderBuilder<()>, ExcelError>
    where
        P: AsRef<Path>,
        L: ReadListener<()> + 'static,
    {
        let mut excel_reader_builder = ExcelReaderBuilder::default()
            .file_with_path_name(path)
            .await?;
        excel_reader_builder.register_read_listener(read_listener);

        Ok(excel_reader_builder)
    }

    async fn read_with_path_head_and_listener<T>(
        path: impl AsRef<Path>,
        read_listener: impl ReadListener<T> + 'static,
    ) -> Result<ExcelReaderBuilder<T>, ExcelError> {
        let mut excel_reader_builder = ExcelReaderBuilder::default()
            .file_with_path_name(path)
            .await?;
        excel_reader_builder.register_read_listener(read_listener);

        Ok(excel_reader_builder)
    }

    fn read_with_r<R>(reader: R) -> ExcelReaderBuilder<()>
    where
        R: Read + 'static,
    {
        ExcelReaderBuilder::default().reader(reader)
    }

    fn read_with_r_and_listener<R, L>(reader: R, read_listener: L) -> ExcelReaderBuilder<()>
    where
        R: Read + 'static,
        L: ReadListener<()> + 'static,
    {
        let mut excel_reader_builder = ExcelReaderBuilder::default().reader(reader);
        excel_reader_builder.register_read_listener(read_listener);

        excel_reader_builder
    }

    fn read_with_r_head_and_listener<T>(
        reader: impl Read + 'static,
        read_listener: impl ReadListener<T> + 'static,
    ) -> ExcelReaderBuilder<T> {
        let mut excel_reader_builder = ExcelReaderBuilder::default().reader(reader);
        excel_reader_builder.register_read_listener(read_listener);

        excel_reader_builder
    }

    fn read_sheet() -> ExcelReaderSheetBuilder<()> {
        ExcelReaderSheetBuilder::default()
    }

    fn read_sheet_with_no(sheet_no: u32) -> ExcelReaderSheetBuilder<()> {
        ExcelReaderSheetBuilder::default().sheet_no(sheet_no)
    }

    fn read_sheet_with_name<S>(sheet_name: S) -> ExcelReaderSheetBuilder<()>
    where
        S: ToString,
    {
        ExcelReaderSheetBuilder::default().sheet_name(sheet_name.to_string())
    }

    fn read_sheet_with_name_and_no<S>(sheet_name: S, sheet_no: u32) -> ExcelReaderSheetBuilder<()>
    where
        S: ToString,
    {
        ExcelReaderSheetBuilder::default()
            .sheet_no(sheet_no)
            .sheet_name(sheet_name.to_string())
    }

    /// Build excel the 'read_sheet_with_details'
    fn read_sheet_with_details<S>(
        sheet_name: S,
        sheet_no: u32,
        num_rows: u32,
    ) -> ExcelReaderSheetBuilder<()>
    where
        S: ToString,
    {
        ExcelReaderSheetBuilder::default()
            .sheet_no(sheet_no)
            .sheet_name(sheet_name.to_string())
            .num_rows(num_rows)
    }
}
