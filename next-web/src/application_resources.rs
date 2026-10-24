use next_web_core::io::ResourceLoader;

pub trait ApplicationResources
where
    Self: ResourceLoader,
{
}
