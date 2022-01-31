mod function;

use function::*;
use futures::stream::AbortHandle;
use neon::prelude::*;
use once_cell::sync::OnceCell;
use std::cell::RefCell;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tonic::{transport::Server, Request, Response, Status};

fn start_func(mut cx: FunctionContext) -> JsResult<JsBox<FunctionServer>> {
    let addr = "[::1]:50051".parse().unwrap();
    let func = Function {
        channel: cx.channel(),
        callback: Arc::new(cx.argument::<JsFunction>(0)?.root(&mut cx)),
    };
    let serve = Server::builder()
        .add_service(function_server::FunctionServer::new(func))
        .serve(addr);

    let (abortable, handle) = futures::future::abortable(serve);

    runtime(&mut cx)?.spawn(abortable);

    let server = FunctionServer {
        handle: RefCell::new(Some(handle)),
    };

    Ok(cx.boxed(server))
}

fn stop_func(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let server = cx.argument::<JsBox<FunctionServer>>(0)?;

    *server.handle.borrow_mut() = None;

    Ok(cx.undefined())
}

struct FunctionServer {
    handle: RefCell<Option<AbortHandle>>,
}

impl Finalize for FunctionServer {}

pub struct Function {
    channel: Channel,
    callback: Arc<Root<JsFunction>>,
}

#[tonic::async_trait]
impl function_server::Function for Function {
    async fn process(
        &self,
        request: Request<FunctionRequest>,
    ) -> Result<Response<FunctionResponse>, Status> {
        println!("Got a request: {:?}", request);

        let request = request.into_inner();
        let callback = self.callback.clone();
        let (tx, rx) = futures::channel::oneshot::channel::<String>();

        let _ = self.channel.try_send(move |mut cx| {
            let this = cx.undefined();
            let arg = cx.string(request.value);
            let value = callback
                .to_inner(&mut cx)
                .call(&mut cx, this, vec![arg.upcast()])?
                .downcast_or_throw::<JsString, _>(&mut cx)?
                .value(&mut cx);

            let _ = tx.send(value);

            Ok(())
        });

        let value = rx
            .await
            .map_err(|err| Status::internal(format!("Failed to call JavaScript: {:?}", err)))?;

        let reply = FunctionResponse { value: value };
        Ok(Response::new(reply))
    }
}

fn runtime<'a, C: Context<'a>>(cx: &mut C) -> NeonResult<&'static Runtime> {
    static RUNTIME: OnceCell<Runtime> = OnceCell::new();

    RUNTIME.get_or_try_init(|| Runtime::new().or_else(|err| cx.throw_error(err.to_string())))
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("startFunc", start_func)?;
    cx.export_function("stopFunc", stop_func)?;
    Ok(())
}
