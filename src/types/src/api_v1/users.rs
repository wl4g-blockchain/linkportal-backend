// SPDX-License-Identifier: GNU GENERAL PUBLIC LICENSE Version 3
//
// Copyleft (c) 2024 James Wong. This file is part of James Wong.
// is free software: you can redistribute it and/or modify it under
// the terms of the GNU General Public License as published by the
// Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// James Wong is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with James Wong.  If not, see <https://www.gnu.org/licenses/>.
//
// IMPORTANT: Any software that fully or partially contains or uses materials
// covered by this license must also be released under the GNU GPL license.
// This includes modifications and derived works.

use crate::{sys::user::User, BaseBean, PageResponse};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Deserialize, Clone, Debug, PartialEq, Validate, utoipa::ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct QueryUserApiV1Request {
    #[validate(length(min = 1, max = 64))]
    pub name: Option<String>,
    #[validate(email)]
    #[validate(length(min = 1, max = 64))]
    pub email: Option<String>,
    #[validate(length(min = 1, max = 15))]
    pub phone: Option<String>,
    #[validate(length(min = 1, max = 64))]
    pub oidc_claims_sub: Option<String>,
    #[validate(length(min = 1, max = 64))]
    pub oidc_claims_name: Option<String>,
    #[validate(length(min = 1, max = 64))]
    pub oidc_claims_email: Option<String>,
    #[validate(length(min = 1, max = 64))]
    pub github_claims_sub: Option<String>,
    #[validate(length(min = 1, max = 64))]
    pub github_claims_name: Option<String>,
    #[validate(length(min = 1, max = 64))]
    pub github_claims_email: Option<String>,
    #[validate(length(min = 1, max = 64))]
    pub google_claims_sub: Option<String>,
    #[validate(length(min = 1, max = 64))]
    pub google_claims_name: Option<String>,
    #[validate(length(min = 1, max = 64))]
    pub google_claims_email: Option<String>,
    #[validate(length(min = 1, max = 64))]
    pub ethers_address: Option<String>,
}

impl QueryUserApiV1Request {
    pub fn to_user(&self) -> User {
        User {
            base: BaseBean::new_with_by(None, None, None),
            name: Some(self.name.clone().unwrap_or_default()),
            email: Some(self.email.clone().unwrap_or_default()),
            phone: self.phone.clone(),
            password: None,
            oidc_claims_sub: None,
            oidc_claims_name: None,
            oidc_claims_email: None,
            github_claims_sub: None,
            github_claims_name: None,
            github_claims_email: None,
            google_claims_sub: None,
            google_claims_name: None,
            google_claims_email: None,
            ethers_address: None,
            lang: None,
        }
    }
}

#[derive(Serialize, Clone, Debug, PartialEq, utoipa::ToSchema)]
pub struct QueryUserApiV1Response {
    pub page: Option<PageResponse>,
    pub data: Option<Vec<User>>,
}

impl QueryUserApiV1Response {
    pub fn new(page: PageResponse, data: Vec<User>) -> Self {
        QueryUserApiV1Response {
            page: Some(page),
            data: Some(data),
        }
    }
}

#[derive(Deserialize, Clone, Debug, PartialEq, Validate, utoipa::ToSchema)]
pub struct SaveUserApiV1Request {
    pub id: Option<i64>,
    #[validate(length(min = 1, max = 64))]
    pub name: Option<String>,
    #[validate(email)]
    #[validate(length(min = 1, max = 64))]
    pub email: Option<String>,
    #[validate(length(min = 1, max = 15))]
    pub phone: Option<String>,
    #[validate(length(min = 1, max = 512))]
    pub password: Option<String>,
    #[validate(length(min = 1, max = 64))]
    pub oidc_claims_sub: Option<String>,
    #[validate(length(min = 1, max = 64))]
    pub oidc_claims_name: Option<String>,
    #[validate(length(min = 1, max = 64))]
    pub oidc_claims_email: Option<String>,
    #[validate(length(min = 1, max = 64))]
    pub github_claims_sub: Option<String>,
    #[validate(length(min = 1, max = 64))]
    pub github_claims_name: Option<String>,
    #[validate(length(min = 1, max = 64))]
    pub github_claims_email: Option<String>,
    #[validate(length(min = 1, max = 64))]
    pub google_claims_sub: Option<String>,
    #[validate(length(min = 1, max = 64))]
    pub google_claims_name: Option<String>,
    #[validate(length(min = 1, max = 64))]
    pub google_claims_email: Option<String>,
    #[validate(length(min = 1, max = 64))]
    pub ethers_address: Option<String>,
    #[validate(length(min = 1, max = 64))]
    pub lang: Option<String>,
}

impl SaveUserApiV1Request {
    pub fn to_user(&self) -> User {
        User {
            base: BaseBean::new_with_id(self.id),
            name: self.name.clone(),
            email: self.email.clone(),
            phone: self.phone.clone(),
            password: self.password.clone(),
            oidc_claims_sub: self.oidc_claims_sub.clone(),
            oidc_claims_name: self.oidc_claims_name.clone(),
            oidc_claims_email: self.oidc_claims_email.clone(),
            github_claims_sub: self.github_claims_sub.clone(),
            github_claims_name: self.github_claims_name.clone(),
            github_claims_email: self.github_claims_email.clone(),
            google_claims_sub: self.google_claims_sub.clone(),
            google_claims_name: self.google_claims_name.clone(),
            google_claims_email: self.google_claims_email.clone(),
            ethers_address: self.ethers_address.clone(),
            lang: self.lang.clone(),
        }
    }
}

#[derive(Serialize, Clone, Debug, PartialEq, utoipa::ToSchema)]
pub struct SaveUserApiV1Response {
    pub id: i64,
}

impl SaveUserApiV1Response {
    pub fn new(id: i64) -> Self {
        SaveUserApiV1Response { id }
    }
}

#[derive(Deserialize, Clone, Debug, PartialEq, Validate, utoipa::ToSchema)]
pub struct DeleteUserApiV1Request {
    pub id: i64,
}

#[derive(Serialize, Clone, Debug, PartialEq, utoipa::ToSchema)]
pub struct DeleteUserApiV1Response {
    pub count: u64,
}

impl DeleteUserApiV1Response {
    pub fn new(count: u64) -> Self {
        DeleteUserApiV1Response { count }
    }
}
