pub mod manual;
pub mod negotiator;

use crate::errors::LinkError;

fn wrap_panic_on_err<T>(
    result: impl futures::Future<Output = Result<T, LinkError>>,
) -> impl futures::Future<Output = ()> {
    async move {
        if let Err(err) = result.await {
            panic!("Link error: {:?}", err);
        }
    }
}
