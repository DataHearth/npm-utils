#[macro_export]
macro_rules! hashmap {
    () => {
        std::collections::HashMap::new()
    };

    ($( ($key:expr, $value:expr) ),* $(,)?) => {
        {
            let mut map = std::collections::HashMap::new();
            $(
                map.insert($key, $value);
            )*
            map
        }
    };
}

#[macro_export]
macro_rules! headers {
    ($( ($key:expr, $value:expr) ),* $(,)?) => {
        {
            let mut headers = reqwest::header::HeaderMap::new();
            $(
                headers.insert(
                    $key,
                    $value
                        .parse::<HeaderValue>()
                        .map_err(|e| CustomErrors::HttpHeaderParse(e.to_string()))?,
                );
            )*
            headers
        }
    };
}

#[macro_export]
macro_rules! hashmap_ext_cond {
    ($( ($cond:expr, $hashmap:expr) ),+ $(,)?) => {
        {
            let mut map = std::collections::HashMap::new();
            $(
                if $cond {
                    map.extend($hashmap);
                }
            )+
            map
        }
    };
}
