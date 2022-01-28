mod function;

use function::*;
use neon::prelude::*;
use once_cell::sync::OnceCell;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tonic::{transport::Server, Request, Response, Status};

fn start_func(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let addr = "[::1]:50051".parse().unwrap();
    let func = Function {
        channel: Arc::new(cx.channel()),
        callback: Arc::new(cx.argument::<JsFunction>(0)?.root(&mut cx)),
    };
    let svc = function_server::FunctionServer::new(func);

    let rt = runtime(&mut cx)?;
    let result = rt.block_on(Server::builder().add_service(svc).serve(addr));

    Ok(cx.undefined())
}

fn runtime<'a, C: Context<'a>>(cx: &mut C) -> NeonResult<&'static Runtime> {
    static RUNTIME: OnceCell<Runtime> = OnceCell::new();

    RUNTIME.get_or_try_init(|| Runtime::new().or_else(|err| cx.throw_error(err.to_string())))
}

pub struct Function {
    channel: Arc<Channel>,
    callback: Arc<Root<JsFunction>>,
}

#[tonic::async_trait]
impl function_server::Function for Function {
    async fn process(
        &self,
        request: Request<FunctionRequest>,
    ) -> Result<Response<FunctionResponse>, Status> {
        println!("Got a request: {:?}", request);

        let request_val = request.into_inner().value;
        let channel = self.channel.clone();

        let (sender, receiver) = tokio::sync::oneshot::channel::<String>();

        let f = self.callback.clone();
        println!("before channel");
        channel.send(move |mut cx| {
            println!("inside channel");

            let f = f.to_inner(&mut cx);
            let this = cx.undefined();
            let arg: Handle<JsString> = cx.string(request_val);
            let value: Handle<JsValue> = f.call(&mut cx, this, vec![arg.upcast()])?;
            let value: Handle<JsString> = value.downcast_or_throw(&mut cx)?;

            println!("{}", value.value(&mut cx));

            sender.send(value.value(&mut cx)).unwrap();
            Ok(())
        });
        println!("after channel");

        tokio::select! {
            val = receiver => {
        let reply = FunctionResponse { value: val.unwrap() };
        Ok(Response::new(reply))
            }
        }
    }
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("startFunc", start_func)?;
    Ok(())
}
