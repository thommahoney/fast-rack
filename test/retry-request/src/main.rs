use fastly::{Error, Request, Response};
use fast_rack::{FastRack, Middleware, RackError};

struct RetryRequest {
    remaining: u8,
}

impl RetryRequest {
    fn new(retries: u8) -> Self {
        RetryRequest {
            remaining: retries,
        }
    }
}

impl Middleware for RetryRequest {
    fn req(&mut self, _req: &mut Request) -> Result<(), RackError> {
        Ok(())
    }

    fn resp(&mut self, _resp: &mut Response) -> Result<(), RackError> {
        // TODO: inspect response status?
        let should_retry = self.remaining > 0;

        if should_retry {
            self.remaining -= 1;

            return Err(RackError::Retry)
        }

        Ok(())
    }
}

#[fastly::main]
fn main(mut request: Request) -> Result<Response, Error> {
    let mut rack = FastRack::new();

    let mut retry_request = RetryRequest::new(2);

    rack.add(&mut retry_request);

    rack.run(&mut request)
}
