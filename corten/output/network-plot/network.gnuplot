set term postscript color eps enhanced 22
set output 'network.eps'
#load "styles.inc"
set size 1.4,1

set xlabel "Time (time units)"
set ylabel "CDF \nof messages received"
#set yrange[0:600]
#set ytics 100	
#set mytics 5
#set xrange[0:300]
set xtics 100
set key bottom right 

plot "latency-constant-cdf.dat" using 1:2 with linespoints ls 1 lc rgb "red" title "Constant Latency", \
	"latency-matrix-cdf.dat" using 1:2 with linespoints ls 2 lc rgb "blue" title "Latency Matrix"


!epstopdf "network.eps"
!rm "network.eps"
