MatrixNetwork="static LATENCY_FILE: \&str = \"output/network-plot/latency-matrix.dat\";"
ConstantNetwork="static LATENCY_FILE: \&str = \"output/network-plot/latency-constant.dat\";"


sed -i '' 's,'"$MatrixNetwork"','"$ConstantNetwork"',' src/simulation/mod.rs

cargo test test_constant_network --features test_network 2> /dev/null;

sed -i '' 's,'"$ConstantNetwork"','"$MatrixNetwork"',' src/simulation/mod.rs

cargo test test_matrix_network --features test_network 2> /dev/null;


#######################

cd output

./script_plot_network.sh
