extern crate libpq_sys;

use std::ffi::CString;
use std::os::raw as libc;
use std::ptr;

use super::result::PgResult;
use crate::pg::PgTypeMetadata;
use crate::result::QueryResult;

use super::raw::RawConnection;

pub(crate) struct Statement {
    name: CString,
    param_formats: Vec<libc::c_int>,
}

impl Statement {
    pub(super) fn execute(
        &self,
        raw_connection: &mut RawConnection,
        param_data: &[Option<Vec<u8>>],
    ) -> QueryResult<PgResult> {
        let params_pointer = param_data
            .iter()
            .map(|data| {
                data.as_ref()
                    .map(|d| d.as_ptr() as *const libc::c_char)
                    .unwrap_or(ptr::null())
            })
            .collect::<Vec<_>>();
        let param_lengths = param_data
            .iter()
            .map(|data| data.as_ref().map(|d| d.len() as libc::c_int).unwrap_or(0))
            .collect::<Vec<_>>();
        let internal_res = unsafe {
            raw_connection.exec_prepared(
                self.name.as_ptr(),
                params_pointer.len() as libc::c_int,
                params_pointer.as_ptr(),
                param_lengths.as_ptr(),
                self.param_formats.as_ptr(),
                1,
            )
        };

        PgResult::new(internal_res?, raw_connection)
    }

    pub(super) fn prepare(
        raw_connection: &mut RawConnection,
        sql: &str,
        name: Option<&str>,
        param_types: &[PgTypeMetadata],
    ) -> QueryResult<Self> {
        let name = CString::new(name.unwrap_or(""))?;
        let sql = CString::new(sql)?;
        let param_types_vec = param_types
            .iter()
            .map(|x| x.oid())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| crate::result::Error::SerializationError(Box::new(e)))?;

        let internal_result = unsafe {
            raw_connection.prepare(
                name.as_ptr(),
                sql.as_ptr(),
                param_types.len() as libc::c_int,
                param_types_to_ptr(Some(&param_types_vec)),
            )
        };
        PgResult::new(internal_result?, raw_connection)?;

        Ok(Statement {
            name,
            param_formats: vec![1; param_types.len()],
        })
    }
}

fn param_types_to_ptr(param_types: Option<&Vec<u32>>) -> *const libpq_sys::Oid {
    param_types
        .map(|types| types.as_ptr())
        .unwrap_or(ptr::null())
}
