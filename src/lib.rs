use ntex::http::uri::{PathAndQuery, Uri};
use ntex::util::Bytes;
use ntex::{forward_poll, forward_ready, forward_shutdown, Service, ServiceCtx};
use ntex::web::{WebRequest, WebResponse};
use regex::Regex;

#[derive(Debug, Default)]
pub struct RemoveTrailingSlash;

impl<S> ntex::Middleware<S> for RemoveTrailingSlash
{
    type Service = RemoveTrailingSlashMiddleware<S>;

    fn create(&self, service: S) -> Self::Service {
        RemoveTrailingSlashMiddleware {
            service,
            merge_slash: Regex::new("//+").unwrap()
        }
    }
}

#[derive(Debug)]
pub struct RemoveTrailingSlashMiddleware<S> {
    service: S,
    merge_slash:Regex
}

impl<S, E> Service<WebRequest<E>> for RemoveTrailingSlashMiddleware<S>
where
    S: Service<WebRequest<E>, Response = WebResponse>
{
    type Response = WebResponse;
    type Error = S::Error;

    forward_poll!(service);
    forward_ready!(service);
    forward_shutdown!(service);

    async fn call(&self, mut req: WebRequest<E>, ctx: ServiceCtx<'_, Self>) -> Result<Self::Response, Self::Error> {
        let head = req.head_mut();
        let original_path = head.uri.path();

        if !original_path.is_empty() {
            let path=original_path.trim_end_matches('/').to_string();
            let path = self.merge_slash.replace_all(&path, "/");
            let path = if path.is_empty() { "/" } else { path.as_ref() };

            if path != original_path {
                let mut parts = head.uri.clone().into_parts();
                let query = parts.path_and_query.as_ref().and_then(|pq| pq.query());

                let path = match query {
                    Some(q) => Bytes::from(format!("{}?{}", path, q)),
                    None => Bytes::copy_from_slice(path.as_bytes()),
                };

                parts.path_and_query = Some(PathAndQuery::from_maybe_shared(path).unwrap());

                let uri = Uri::from_parts(parts).unwrap();
                req.match_info_mut().set(uri.clone());
                req.head_mut().uri = uri;
            }
        }

        ctx.call(&self.service, req).await
    }
}
