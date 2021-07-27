use fastly::{Error, Request, Response};
use fast_rack::{FastRack, Middleware, RackError};

struct SynetheticResponse {
    body: &'static str,
}

impl Middleware for SynetheticResponse {
    fn req(&self, _req: &mut Request) -> Result<(), RackError> {
        let response = Response::from_body(self.body);
        Err(RackError::Synthetic(response))
    }

    fn resp(&self, _resp: &mut Response) -> Result<(), RackError> {
        Ok(())
    }
}

#[fastly::main]
fn main(mut request: Request) -> Result<Response, Error> {
    let mut rack = FastRack::new();

    rack.add(&SynetheticResponse {
        body: "foo",
    });

    rack.run(&mut request)
}
