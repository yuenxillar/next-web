use std::vec;

use indexmap::{map::Entry, IndexMap};
use next_web_core::traits::filter::{http_filter::HttpFilter, http_filter_chain::HttpFilterChain};
use tracing::debug;

use crate::web::filter::mgt::{
    default_filter::DefaultFilter, filter_chain_manager::FilterChainManager,
    named_filter_list::NamedFilterList, simple_mamed_filter_list::SimpleNamedFilterList,
};

pub struct DefaultFilterChainManager {
    pub(crate) filters: IndexMap<String, Box<dyn HttpFilter>>,
    pub(crate) filter_chains: IndexMap<String, Box<dyn NamedFilterList>>,
    pub(crate) global_filter_names: Vec<String>,
}

impl DefaultFilterChainManager {
    pub fn get_filters(&mut self) -> &mut IndexMap<String, Box<dyn HttpFilter>> {
        &mut self.filters
    }

    pub fn get_filter(&mut self, filter_name: &str) -> Option<&dyn HttpFilter> {
        self.filters.get(filter_name).map(|f| f.as_ref())
    }

    // pub fn get_mut_chain<'a>(&'a mut self, chain_name: &str) -> Option<&'a mut dyn NamedFilterList> {
    //     self.filter_chains.get_mut(chain_name).map(|s| s.as_mut())
    // }

    fn _get_box_filter(&mut self, filter_name: &str) -> Option<Box<dyn HttpFilter>> {
        self.filters.get(filter_name).map(Clone::clone)
    }

    pub fn add_filter(
        &mut self,
        name: impl ToString,
        filter: Box<dyn HttpFilter>,
        overwrite: bool,
    ) {
        let name = name.to_string();
        if !self.filters.contains_key(&name) || overwrite {
            self.filters.insert(name, filter);
        }
    }

    // pub fn init_filter(&mut self, filter: impl ToString) {}

    fn ensure_chain(&mut self, chain_name: &str) -> &mut dyn NamedFilterList {
        match self.filter_chains.entry(chain_name.to_string()) {
            Entry::Occupied(entry) => entry.into_mut().as_mut(),
            Entry::Vacant(entry) => {
                let chain = SimpleNamedFilterList::new(chain_name);
                entry.insert(Box::new(chain)).as_mut()
            }
        }
    }

    fn split_chain_definition<'a>(&self, chain_definition: &'a str) -> Vec<&'a str> {
        chain_definition
            .trim()
            .split(",")
            .map(|s| s.trim())
            .collect()
    }

    fn to_name_config_pair(&self, token: &str) -> Vec<String> {
        let parts = token.splitn(2, '[').collect::<Vec<&str>>();
        let name = parts[0].trim();

        assert!(
            !name.is_empty(),
            "Filter name not found for filter chain definition token: {}",
            token
        );
        let mut config = "";
        if parts.len() == 2 {
            config = parts[1].trim();

            // if there was an open bracket, it assumed there is a closing bracket, so strip it too:
            config = &config[0..config.len() - 1];
            config = config.trim();

            // backwards compatibility prior to implementing SHIRO-205:
            // prior to SHIRO-205 being implemented, it was common for end-users to quote the config inside brackets
            // if that config required commas.  We need to strip those quotes to get to the interior quoted definition
            // to ensure any existing quoted definitions still function for end users:
            if !config.is_empty() && config.starts_with("\"") && config.ends_with("\"") {
                let mut stripped = &config[1..config.len() - 1];
                stripped = stripped.trim();

                // if the stripped value does not have any internal quotes, we can assume that the entire config was
                // quoted and we can use the stripped value.
                if !stripped.is_empty() && stripped.find("\"").map(|_n| false).unwrap_or(true) {
                    config = stripped;
                }

                // else:
                // the remaining config does have internal quotes, so we need to assume that each comma delimited
                // pair might be quoted, in which case we need the leading and trailing quotes that we stripped
                // So we ignore the stripped value.
            }
        }

        vec![name.to_string(), config.to_string()]
    }

    fn add_default_filters(&mut self) {
        for filter in DefaultFilter::values() {
            self.add_filter(filter.name(), filter.new_instance(), false);
        }
    }
}

impl FilterChainManager for DefaultFilterChainManager {
    fn get_filters(&mut self) -> &mut IndexMap<String, Box<dyn HttpFilter>> {
        &mut self.filters
    }

    fn get_chain(&self, chain_name: &str) -> Option<&dyn NamedFilterList> {
        self.filter_chains.get(chain_name).map(|s| s.as_ref())
    }

    fn has_chains(&self) -> bool {
        !self.filter_chains.is_empty()
    }

    fn get_chain_names(&self) -> Vec<&str> {
        self.filter_chains.keys().map(|s| s.as_str()).collect()
    }

    fn add_filter(&mut self, name: String, filter: Box<dyn HttpFilter>) {
        self.add_filter(name, filter, false);
    }

    fn create_chain(&mut self, chain_name: String, chain_definition: String) {
        assert!(
            !chain_name.is_empty(),
            "chain_name cannot be null or empty."
        );
        assert!(
            !chain_definition.is_empty(),
            "chain_definition cannot be null or empty."
        );
        debug!(
            "Creating chain [{}] with global filters  and from String definition [{:?}]",
            chain_name, self.global_filter_names
        );

        // first add each of global filters
        if !self.global_filter_names.is_empty() {
            for filter_name in self.global_filter_names.clone() {
                self.add_to_chain(&chain_name, filter_name, None);
            }
        }

        //parse the value by tokenizing it to get the resulting filter-specific config entries
        //
        //e.g. for a value of
        //
        //     "authc, roles[admin,user], perms[file:edit]"
        //
        // the resulting token array would equal
        //
        //     { "authc", "roles[admin,user]", "perms[file:edit]" }
        //
        let filter_tokens = self.split_chain_definition(&chain_definition);

        // each token is specific to each filter.
        // strip the name and extract any filter-specific config between brackets [ ]
        for token in filter_tokens {
            let mut name_config_pair = self.to_name_config_pair(token);

            //now we have the filter name, path and (possibly null) path-specific config.  Let's apply them:
            let pair1 = name_config_pair.remove(0);
            let pair2 = name_config_pair.remove(0);
            self.add_to_chain(&chain_name, pair1, Some(&pair2));
        }
    }

    fn create_default_chain(&mut self, chain_name: String) {
        // only create the defaultChain if we don't have a chain with this name already
        // (the global filters will already be in that chain)
        if !self.get_chain_names().contains(&chain_name.as_str())
            && !self.global_filter_names.is_empty()
        {
            println!("global_filter_names: {:?}", self.global_filter_names);
            for filter_name in self.global_filter_names.clone() {
                self.add_to_chain(&chain_name, filter_name, None);
            }
        }
    }

    fn add_to_chain(
        &mut self,
        chain_name: &str,
        filter_name: String,
        chain_specific_filter_config: Option<&str>,
    ) {
        assert!(
            !chain_name.is_empty(),
            "chain_name cannot be null or empty."
        );
        match self._get_box_filter(&filter_name) {
            None => panic!("There is no filter with name [{filter_name}] to apply to chain [{chain_name}] in the pool of available Filters.  Ensure a
            filter with that name/path has first been registered with the addFilter method(s)."),
            Some(mut filter) => {

                println!("found filter: {}", filter.name());
                if let Some(config) = chain_specific_filter_config {
                    filter.process_path_config(chain_name, config);
                }

                let chain = self.ensure_chain(chain_name);
                chain.get_mut_object().push(filter);
            }

        }
    }

    fn set_global_filters(&mut self, global_filter_names: Vec<String>) {
        if !global_filter_names.is_empty() {
            for filter_name in global_filter_names {
                if self.filters.contains_key(&filter_name) {
                    self.global_filter_names.push(filter_name);
                } else {
                    panic!("There is no filter with name [{filter_name}]
                    to apply to the global filters in the pool of available Filters.  Ensure a
                    filter with that name/path has first been registered with the addFilter method(s).")
                }
            }
        }
    }

    fn proxy(&self, original: &dyn HttpFilterChain, chain_name: &str) -> Box<dyn HttpFilterChain> {
        let chain = self.get_chain(chain_name);
        println!("name: {}", chain_name);
        match chain {
            Some(configured) => configured.proxy(original),
            None => panic!(
                "There is no configured chain under the name/key [{}]",
                chain_name
            ),
        }
    }
}

impl Default for DefaultFilterChainManager {
    fn default() -> Self {
        let mut manager = Self {
            filters: IndexMap::new(),
            filter_chains: IndexMap::new(),
            global_filter_names: Vec::new(),
        };
        manager.add_default_filters();

        manager
    }
}

#[cfg(test)]
mod tests {
    use std::any::Any;

    use next_web_core::traits::filter::http_filter::HttpFilter;

    use crate::web::filter_proxy::FilterProxy;

    #[test]
    fn down() {
        let filter = FilterProxy::default();

        let f = Box::new(filter) as Box<dyn HttpFilter>;

        if let Some(proxy) = (&f as &dyn Any).downcast_ref::<FilterProxy>() {
            println!("url: {:?}", proxy.get_login_url())
        }
    }
}
