
#[cfg(test)]
mod perf_probe {
    use std::hint::black_box;
    use std::time::Instant;

    use crate::filter::gateway_filter::DefaultGatewayFilter;
    use crate::filter::spec::FilterPhase;

    fn filters() -> Vec<DefaultGatewayFilter> {
        ["AddRequestHeader=X-A, 1", "SetRequestHeader=X-B, 2", "RemoveRequestHeader=X-C",
         "PrefixPath=/root", "StripPrefix=1", "SecureHeaders", "AddResponseHeader=X-D, 3",
         "SetStatus=201"]
            .iter()
            .filter_map(|raw| DefaultGatewayFilter::try_from(*raw).ok())
            .collect()
    }

    #[test]
    fn measure_chain_construction() {
        let filters = filters();
        let rounds = 200_000;

        let start = Instant::now();
        for _ in 0..rounds {
            black_box(DefaultGatewayFilter::chain_for(
                &filters,
                FilterPhase::RequestHeader,
            ));
        }
        let per_call = start.elapsed() / rounds;

        println!(
            "chain_for: {} filters, {} ns per request phase",
            filters.len(),
            per_call.as_nanos()
        );

        let prebuilt = std::sync::Arc::from(DefaultGatewayFilter::chain_for(
            &filters,
            FilterPhase::RequestHeader,
        ));
        let start = Instant::now();
        for _ in 0..rounds {
            black_box(&prebuilt);
        }
        let prebuilt_ns = start.elapsed() / rounds;

        println!("prebuilt:  {} ns per request phase", prebuilt_ns.as_nanos());
    }
}
