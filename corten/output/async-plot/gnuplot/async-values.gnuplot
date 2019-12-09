set term postscript color eps enhanced 22
set output '../graphs/async-values.eps'
#load "styles.inc"
set size 1.4,1

set xlabel "Time (time units)"
set ylabel "CDF \nof asynchrony"
set yrange[0.0:1.0]
#set ytics 100	
#set mytics 5
#set xrange[0:3000]
#set xtics 200
set key bottom right

plot "../data/values-cdf/no-async.dat" using 1:2 with linespoints ls 1 lc rgb "red" title "No asynchrony", \
	"../data/values-cdf/uniform-async.dat" using 1:2 with linespoints ls 2 lc rgb "blue" title "Uniform asynchrony", \
	"../data/values-cdf/normal-async.dat" using 1:2 with linespoints ls 3 lc rgb "green" title "Normal asynchrony", \
	"../data/values-cdf/weibull-async.dat" using 1:2 with linespoints ls 4 lc rgb "orange" title "Weibull asynchrony"


!epstopdf "../graphs/async-values.eps"
!rm "../graphs/async-values.eps"
