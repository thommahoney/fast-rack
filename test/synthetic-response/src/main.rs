use fastly::{Error, Request, Response};
use fast_rack::{FastRack, Middleware, RackError};

struct SyntheticResponse {
    body: &'static str,
    status: u16,
}

impl Middleware for SyntheticResponse {
    fn req(&mut self, _req: &mut Request) -> Result<(), RackError> {
        let mut response = Response::from_body(self.body);
        response.set_status(self.status);
        Err(RackError::Synthetic(response))
    }

    fn resp(&mut self, _resp: &mut Response) -> Result<(), RackError> {
        Ok(())
    }
}

#[fastly::main]
fn main(mut request: Request) -> Result<Response, Error> {
    let mut rack = FastRack::new();

    let mut synthetic_response = SyntheticResponse {
        body: "foo",
        status: 418,
    };

    rack.add(&mut synthetic_response);

    rack.run(&mut request)
}
