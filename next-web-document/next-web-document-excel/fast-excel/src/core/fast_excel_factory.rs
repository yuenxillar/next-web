#[cfg(feature = "async")]
use tokio::{
    fs::File as TokioFile,
    io::{AsyncRead, AsyncWrite},
};

#[cfg(not(feature = "async"))]
use std::fs::File as StdFile;
#[cfg(feature = "async")]
use std::path::Path;

#[cfg(feature = "async")]
use crate::core::write::builder::excel_writer_builder::ExcelWriterBuilder;

#[cfg(not(feature = "async"))]
pub trait FastExcelFactory {
    fn write() {}
    fn write_with_file(file: StdFile) {}
    fn read() {}
}

#[cfg(feature = "async")]
pub trait FastExcelFactory {
    fn write() -> ExcelWriterBuilder {
        ExcelWriterBuilder::default()
    }

    fn write_with_file(file: TokioFile) -> ExcelWriterBuilder {
        Self::write_with_file(file)
    }

    fn write_with_head<T>(file: TokioFile, head: T) -> ExcelWriterBuilder {
        let mut excel_writer_builder = Self::write();
        // excel_writer_builder.fi

        excel_writer_builder
    }
    fn write_with_path<P>(path: P)
    where
        P: AsRef<Path>,
    {
    }

    fn write_with_path_and_head<P, T>(path: P, head: T)
    where
        P: AsRef<Path>,
    {
    }

    fn write_with_w<W>(writer: W)
    where
        W: AsyncWrite,
    {
    }

    fn write_with_w_and_head<W, T>(writer: W, head: T)
    where
        W: AsyncWrite,
    {
    }

    fn write_sheet() {}

    fn write_sheet_with_no(sheet_no: u32) {}

    fn write_sheet_with_name<S>(sheet_name: S)
    where
        S: ToString,
    {
    }

    fn write_sheet_with_name_and_no<S>(sheet_name: S, sheet_no: u32)
    where
        S: ToString,
    {
    }

    fn write_table() {}
    fn write_table_with_no(table_no: u32) {}

    fn read() {}
    fn read_with_file(file: TokioFile) {}
    fn read_with_listener<L>(file: TokioFile, listener: L) {}
    fn read_with_head_and_listener<T, L>(file: TokioFile, head: T, listener: L) {}

    fn read_with_path<P>(path: P)
    where
        P: AsRef<Path>,
    {
    }
    fn read_with_path_and_listener<P, L>(path: P, listener: L)
    where
        P: AsRef<Path>,
    {
    }

    fn read_with_path_head_and_listener<P, T, L>(path: P, head: T, listener: L)
    where
        P: AsRef<Path>,
    {
    }

    fn read_with_r<R>(reader: R)
    where
        R: AsyncRead,
    {
    }

    fn read_with_r_and_listener<R, L>(reader: R, listener: L)
    where
        R: AsyncRead,
    {
    }

    fn read_with_r_head_and_listener<R, T, L>(reader: R, head: T, listener: L)
    where
        R: AsyncRead,
    {
    }

    fn read_sheet() {}
    fn read_sheet_with_no(sheet_no: u32) {}
    fn read_sheet_with_name<S>(sheet_name: S)
    where
        S: ToString,
    {
    }
    fn read_sheet_with_name_and_no<S>(sheet_name: S, sheet_no: u32)
    where
        S: ToString,
    {
    }
    fn read_sheet_with_name_no_and_rows<S>(sheet_name: S, sheet_no: u32, num_rows: u32)
    where
        S: ToString,
    {
    }
}
