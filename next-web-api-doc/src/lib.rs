//! Rust implementation of Openapi Spec V3.1.

pub use paste::paste;
pub use utoipa::{
    openapi, {OpenApi, Path, ToSchema},
};

/// Wrapper type for [`utoipa::openapi::path::Paths`] and [`axum::routing::MethodRouter`].
///
/// This is used with [`OpenApiRouter::routes`] method to register current _`paths`_ to the
/// [`utoipa::openapi::OpenApi`] of [`OpenApiRouter`] instance.
///
/// See [`routes`][routes] for usage.
///
/// [routes]: ../macro.routes.html
pub type UtoipaMethodRouter = (
    Vec<(
        String,
        utoipa::openapi::RefOr<utoipa::openapi::schema::Schema>,
    )>,
    utoipa::openapi::path::Paths,
);

/// A wrapper struct for [`axum::Router`] and [`utoipa::openapi::OpenApi`] for composing handlers
/// and services with collecting OpenAPI information from the handlers.
///
/// This struct provides pass through implementation for most of the [`axum::Router`] methods and
/// extends capabilities for few to collect the OpenAPI information. Methods that are not
/// implemented can be easily called after converting this router to [`axum::Router`] by
/// [`Into::into`].
///
/// # Examples
///
/// _**Create new [`OpenApiRouter`] with default values populated from cargo environment variables.**_
/// ```rust
/// # use utoipa_axum::router::OpenApiRouter;
/// let _: OpenApiRouter = OpenApiRouter::new();
/// ```
///
/// _**Instantiate a new [`OpenApiRouter`] with new empty [`utoipa::openapi::OpenApi`].**_
/// ```rust
/// # use utoipa_axum::router::OpenApiRouter;
/// let _: OpenApiRouter = OpenApiRouter::default();
/// ```
#[derive(Debug, Clone)]
pub struct OpenApiRouter(utoipa::openapi::OpenApi);

impl OpenApiRouter {
    /// Instantiate a new [`OpenApiRouter`] with default values populated from cargo environment
    /// variables. This creates an `OpenApi` similar of creating a new `OpenApi` via
    /// `#[derive(OpenApi)]`
    ///
    /// If you want to create [`OpenApiRouter`] with completely empty [`utoipa::openapi::OpenApi`]
    /// instance, use [`OpenApiRouter::default()`].
    pub fn new() -> OpenApiRouter {
        use utoipa::OpenApi;
        #[derive(OpenApi)]
        struct Api;

        Self::with_openapi(Api::openapi())
    }

    /// Instantiates a new [`OpenApiRouter`] with given _`openapi`_ instance.
    ///
    /// This function allows using existing [`utoipa::openapi::OpenApi`] as source for this router.
    ///
    /// # Examples
    ///
    /// _**Use derived [`utoipa::openapi::OpenApi`] as source for [`OpenApiRouter`].**_
    /// ```rust
    /// # use utoipa::OpenApi;
    /// # use utoipa_axum::router::OpenApiRouter;
    /// #[derive(utoipa::ToSchema)]
    /// struct Todo {
    ///     id: i32,
    /// }
    /// #[derive(utoipa::OpenApi)]
    /// #[openapi(components(schemas(Todo)))]
    /// struct Api;
    ///
    /// let mut router: OpenApiRouter = OpenApiRouter::with_openapi(Api::openapi());
    /// ```
    pub fn with_openapi(openapi: utoipa::openapi::OpenApi) -> Self {
        Self(openapi)
    }

    /// Register [`UtoipaMethodRouter`] content created with [`routes`][routes] macro to `self`.
    ///
    /// Paths of the [`UtoipaMethodRouter`] will be extended to [`utoipa::openapi::OpenApi`] and
    /// [`axum::routing::MethodRouter`] will be added to the [`axum::Router`].
    ///
    /// [routes]: ../macro.routes.html
    pub fn routes(mut self, (schemas, paths): UtoipaMethodRouter) -> Self {
        // add or merge current paths to the OpenApi
        for (path, item) in paths.paths {
            if let Some(it) = self.0.paths.paths.get_mut(&path) {
                it.merge_operations(item);
            } else {
                self.0.paths.paths.insert(path, item);
            }
        }

        let components = self
            .0
            .components
            .get_or_insert(utoipa::openapi::Components::new());
        components.schemas.extend(schemas);

        Self(self.0)
    }

    /// Nest `router` to `self` under given `path`. Router routes will be nested with
    /// [`axum::Router::nest`].
    ///
    /// This method expects [`OpenApiRouter`] instance in order to nest OpenApi paths and router
    /// routes. If you wish to use [`axum::Router::nest`] you need to first convert this instance
    /// to [`axum::Router`] _(`let _: Router = OpenApiRouter::new().into()`)_.
    ///
    /// # Examples
    ///
    /// _**Nest two routers.**_
    /// ```rust
    /// # use utoipa_axum::{routes, PathItemExt, router::OpenApiRouter};
    /// #[utoipa::path(get, path = "/search")]
    /// async fn search() {}
    ///
    /// let search_router = OpenApiRouter::new()
    ///     .routes(utoipa_axum::routes!(search));
    ///
    /// let router: OpenApiRouter = OpenApiRouter::new()
    ///     .nest("/api", search_router);
    /// ```
    pub fn nest(self, path: &str, router: OpenApiRouter) -> Self {
        // from axum::routing::path_router::path_for_nested_route
        // method is private, so we need to replicate it here
        fn path_for_nested_route(prefix: &str, path: &str) -> String {
            let path = if path.is_empty() { "/" } else { path };
            debug_assert!(prefix.starts_with('/'));

            if prefix.ends_with('/') {
                format!("{prefix}{}", path.trim_start_matches('/'))
            } else if path == "/" {
                prefix.into()
            } else {
                format!("{prefix}{path}")
            }
        }

        let api = self.0.nest_with_path_composer(
            path_for_nested_route(path, "/"),
            router.0,
            path_for_nested_route,
        );

        Self(api)
    }

    /// Merge [`utoipa::openapi::path::Paths`] from `router` to `self` and merge [`Router`] routes
    /// and fallback with [`axum::Router::merge`].
    ///
    /// This method expects [`OpenApiRouter`] instance in order to merge OpenApi paths and router
    /// routes. If you wish to use [`axum::Router::merge`] you need to first convert this instance
    /// to [`axum::Router`] _(`let _: Router = OpenApiRouter::new().into()`)_.
    ///
    /// # Examples
    ///
    /// _**Merge two routers.**_
    /// ```rust
    /// # use utoipa_axum::{routes, PathItemExt, router::OpenApiRouter};
    /// #[utoipa::path(get, path = "/search")]
    /// async fn search() {}
    ///
    /// let search_router = OpenApiRouter::new()
    ///     .routes(utoipa_axum::routes!(search));
    ///
    /// let router: OpenApiRouter = OpenApiRouter::new()
    ///     .merge(search_router);
    /// ```
    pub fn merge(mut self, router: OpenApiRouter) -> Self {
        self.0.merge(router.0);

        Self(self.0)
    }

    /// Consume `self` returning the [`utoipa::openapi::OpenApi`] instance of the
    /// [`OpenApiRouter`].
    pub fn into_openapi(self) -> utoipa::openapi::OpenApi {
        self.0
    }

    /// Take the [`utoipa::openapi::OpenApi`] instance without consuming the [`OpenApiRouter`].
    pub fn to_openapi(&mut self) -> utoipa::openapi::OpenApi {
        std::mem::take(&mut self.0)
    }

    /// Get reference to the [`utoipa::openapi::OpenApi`] instance of the router.
    pub fn get_openapi(&self) -> &utoipa::openapi::OpenApi {
        &self.0
    }

    /// Get mutable reference to the [`utoipa::openapi::OpenApi`] instance of the router.
    pub fn get_openapi_mut(&mut self) -> &mut utoipa::openapi::OpenApi {
        &mut self.0
    }
}

impl Default for OpenApiRouter {
    fn default() -> Self {
        Self::with_openapi(utoipa::openapi::OpenApiBuilder::new().build())
    }
}

/// not
#[macro_export]
macro_rules! routes {
    ( $handler:path $(, $tail:path)* $(,)? ) => {
        {
            let mut paths = utoipa::openapi::path::Paths::new();
            let mut schemas = Vec::<(String, utoipa::openapi::RefOr<utoipa::openapi::schema::Schema>)>::new();
            let (path, item, types) = $crate::routes!(@resolve_types $handler : schemas);

            paths.add_path_operation(&path, types, item);

            (schemas, paths)
        }
    };
    ( @resolve_types $handler:path : $schemas:tt ) => {
        {
            $crate::paste! {
                let path = $crate::routes!( @path [path()] of $handler );
                let mut operation = $crate::routes!( @path [operation()] of $handler );
                let types = $crate::routes!( @path [methods()] of $handler );
                let tags = $crate::routes!( @path [tags()] of $handler );
                $crate::routes!( @path [schemas(&mut $schemas)] of $handler );
                if !tags.is_empty() {
                    let operation_tags = operation.tags.get_or_insert(Vec::new());
                    operation_tags.extend(tags.iter().map(ToString::to_string));
                }
                (path, operation, types)
            }
        }
    };
    ( @path $op:tt of $part:ident $( :: $tt:tt )* ) => {
        $crate::routes!( $op : [ $part $( $tt )*] )
    };
    ( $op:tt : [ $first:tt $( $rest:tt )* ] $( $rev:tt )* ) => {
        $crate::routes!( $op : [ $( $rest )* ] $first $( $rev)* )
    };
    ( $op:tt : [] $first:tt $( $rest:tt )* ) => {
        $crate::routes!( @inverse $op : $first $( $rest )* )
    };
    ( @inverse $op:tt : $tt:tt $( $rest:tt )* ) => {
        $crate::routes!( @rev $op : $tt [$($rest)*] )
    };
    ( @rev $op:tt : $tt:tt [ $first:tt $( $rest:tt)* ] $( $reversed:tt )* ) => {
        $crate::routes!( @rev $op : $tt [ $( $rest )* ] $first $( $reversed )* )
    };
    ( @rev [$op:ident $( $args:tt )* ] : $handler:tt [] $($tt:tt)* ) => {
        {
            #[allow(unused_imports)]
            use utoipa::{Path, __dev::{Tags, SchemaReferences}};
            $crate::paste! {
                $( $tt :: )* [<__path_ $handler>]::$op $( $args )*
            }
        }
    };
    ( ) => {};
}
