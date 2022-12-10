// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! info endpoint

use crate::model::info::Info;
use actix_web::{web::Json, HttpResponse};

#[allow(clippy::unused_async)]
pub(crate) async fn info() -> HttpResponse {
    HttpResponse::Ok().json(Json(Info::new()))
}

#[cfg(test)]
mod test {
    use super::info;
    use crate::{endpoints::insecure::insecure_config, model::info::Info};
    use actix_web::{
        http::StatusCode,
        test::{init_service, read_body_json, TestRequest},
        App,
    };

    #[actix_rt::test]
    async fn info_works() {
        let resp = info().await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_rt::test]
    async fn info_in_app_works() {
        let mut app = init_service(App::new().configure(insecure_config)).await;

        let resp = TestRequest::get().uri("/info").send_request(&mut app).await;
        assert!(resp.status().is_success());
        let result: Info<String> = read_body_json(resp).await;
        assert_eq!(result.build_timestamp(), env!("VERGEN_BUILD_TIMESTAMP"));
        assert_eq!(result.build_semver(), env!("VERGEN_BUILD_SEMVER"));
    }
}
