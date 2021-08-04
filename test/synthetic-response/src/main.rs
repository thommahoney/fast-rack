use fastly::{Error, Request, Response};
use fast_rack::{FastRack, Middleware, RackError};

struct SyntheticResponse {
    body: &'static str,
    status: u16,
}

impl Middleware for SyntheticResponse {
    fn req(&self, _req: &mut Request) -> Result<(), RackError> {
        let mut response = Response::from_body(self.body);
        response.set_status(self.status);
        Err(RackError::Synthetic(response))
    }

    fn resp(&self, _resp: &mut Response) -> Result<(), RackError> {
        Ok(())
    }
}

#[fastly::main]
fn main(mut request: Request) -> Result<Response, Error> {
    let mut rack = FastRack::new();

    rack.add(&SyntheticResponse {
        body: "foo",
        status: 418,
    });

    rack.run(&mut request)
}
