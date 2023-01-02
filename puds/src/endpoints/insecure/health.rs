// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! health endpoint

use crate::model::health::Response;
use actix_web::{web::Json, HttpResponse};

#[allow(clippy::unused_async)]
pub(crate) async fn health() -> HttpResponse {
    HttpResponse::Ok().json(Json(Response::healthy()))
}

#[cfg(test)]
mod test {
    use super::health;
    use crate::{endpoints::insecure::insecure_config, model::health::Response};
    use actix_web::{
        http::StatusCode,
        test::{init_service, read_body_json, TestRequest},
        App,
    };

    #[actix_rt::test]
    async fn health_works() {
        let resp = health().await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_rt::test]
    async fn health_in_app_works() {
        let app = init_service(App::new().configure(insecure_config)).await;

        let resp = TestRequest::get().uri("/health").send_request(&app).await;
        assert!(resp.status().is_success());
        let result: Response<String> = read_body_json(resp).await;
        assert_eq!(*result.status(), "healthy");
    }
}
