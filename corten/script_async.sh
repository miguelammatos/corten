NormalAsync="static ASYNC_OUTPUT_FILE: \&str = \"output/async-plot/data/original/normal-async.dat\";"
NoAsync="static ASYNC_OUTPUT_FILE: \&str = \"output/async-plot/data/original/no-async.dat\";"
UniformAsync="static ASYNC_OUTPUT_FILE: \&str = \"output/async-plot/data/original/uniform-async.dat\";"
WeibullAsync="static ASYNC_OUTPUT_FILE: \&str = \"output/async-plot/data/original/weibull-async.dat\";"


sed -i '' 's,'"$NormalAsync"','"$NoAsync"',' src/simulation/mod.rs

cargo test test_no_async --features test_async 2> /dev/null;

sed -i '' 's,'"$NoAsync"','"$UniformAsync"',' src/simulation/mod.rs

cargo test test_uniform_async --features test_async 2> /dev/null;

sed -i '' 's,'"$UniformAsync"','"$WeibullAsync"',' src/simulation/mod.rs

cargo test test_weibull_async --features test_async 2> /dev/null;

sed -i '' 's,'"$WeibullAsync"','"$NormalAsync"',' src/simulation/mod.rs

cargo test test_normal_async --features test_async 2> /dev/null;


#######################

cd output

./script_plot_async.sh
