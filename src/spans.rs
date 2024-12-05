use rocket::http::Status;
use rocket::request::FromRequest;
use rocket::request::Outcome;
use rocket::{
    fairing::{Fairing, Info, Kind},
    Data, Request, Response,
};

use tracing::{info_span, Span};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct RequestId<T = String>(pub T);

// Allows a route to access the request id
#[rocket::async_trait]
impl<'r> FromRequest<'r> for RequestId {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, ()> {
        match request.local_cache(|| RequestId::<Option<String>>(None)) {
            RequestId(Some(request_id)) => Outcome::Success(RequestId(request_id.to_owned())),
            RequestId(None) => Outcome::Error((Status::InternalServerError, ())),
        }
    }
}

#[derive(Clone)]
pub struct TracingSpan<T = Span>(pub T);

pub struct TracingFairing;

#[rocket::async_trait]
impl Fairing for TracingFairing {
    fn info(&self) -> Info {
        Info {
            name: "Tracing Fairing",
            kind: Kind::Request | Kind::Response,
        }
    }
    async fn on_request(&self, req: &mut Request<'_>, _data: &mut Data<'_>) {
        let request_id = req
            .headers()
            .get_one("X-Request-Id")
            .map(ToString::to_string)
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        req.local_cache(|| RequestId(Some(request_id.to_owned())));
    }

    async fn on_response<'r>(&self, req: &'r Request<'_>, res: &mut Response<'r>) {
        if let Some(span) = req
            .local_cache(|| TracingSpan::<Option<Span>>(None))
            .0
            .to_owned()
        {
            let _entered_span = span.entered();
            _entered_span.record("http.status_code", res.status().code);

            if let Some(request_id) = &req.local_cache(|| RequestId::<Option<String>>(None)).0 {
                info!("Returning request {} with {}", request_id, res.status());
            }

            drop(_entered_span);
        }

        if let Some(request_id) = &req.local_cache(|| RequestId::<Option<String>>(None)).0 {
            res.set_raw_header("X-Request-Id", request_id);
        }
    }
}

// Allows a route to access the span
#[rocket::async_trait]
impl<'r> FromRequest<'r> for TracingSpan {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, ()> {
        match request.local_cache(|| TracingSpan::<Option<Span>>(None)) {
            TracingSpan(Some(span)) => Outcome::Success(TracingSpan(span.to_owned())),
            TracingSpan(None) => Outcome::Error((Status::InternalServerError, ())),
        }
    }
}
