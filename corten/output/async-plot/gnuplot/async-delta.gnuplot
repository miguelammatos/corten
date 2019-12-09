set term postscript color eps enhanced 22
set output '../graphs/async-delta.eps'
#load "styles.inc"
set size 1.4,1

set xlabel "Time (time units)"
#set ylabel "CDF \nof local method calls"
#set yrange[0.0:1.0]
#set ytics 100	
#set mytics 5
#set xrange[-200:600]
#set xtics 200
set key bottom right

set style fill solid 0.4

plot "../data/delta/no-async.dat" using 1:2 with linespoints ls 1 lc rgb "red" title "No asynchrony", \
	"../data/delta/uniform-async.dat" using 1:2 with linespoints ls 2 lc rgb "blue" title "Uniform asynchrony", \
	"../data/delta/normal-async.dat" using 1:2 with linespoints ls 3 lc rgb "green" title "Normal asynchrony", \
	"../data/delta/weibull-async.dat" using 1:2 with linespoints ls 4 lc rgb "orange" title "Weibull asynchrony"

!epstopdf "../graphs/async-delta.eps"
!rm "../graphs/async-delta.eps"
