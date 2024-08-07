// This file is part of the Tiny Cloud project.
// You can find the source code of every repository here:
//		https://github.com/personal-tiny-cloud
//
// Copyright (C) 2024  hex0x0000
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// Email: hex0x0000@protonmail.com

use actix_web::{HttpResponse, HttpResponseBuilder};
use tcloud_library::error::ErrToResponse;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PluginError {
    #[error("An internal server error occurred.")]
    InternalError(String),
}

impl ErrToResponse for PluginError {
    fn error(&self) -> &'static str {
        "PluginError"
    }

    fn err_type(&self) -> &'static str {
        match self {
            Self::InternalError(_) => stringify!(InternalError),
        }
    }

    fn msg(&self) -> String {
        self.to_string()
    }

    fn http_code(&self) -> HttpResponseBuilder {
        match self {
            Self::InternalError(_) => HttpResponse::InternalServerError(),
        }
    }

    fn handle(&self) {
        let Self::InternalError(err) = self;
        log::error!("An internal server error occurred during authentication: {err}");
    }
}
