set term postscript color eps enhanced 22
set output '../graphs/async1000precision.eps'
#load "styles.inc"
set size 1.4,1

set xlabel "Time (time units)"
set ylabel "1000precision \nof local method calls"
set yrange[0.0:1.0]
#set ytics 100	
#set mytics 5
set xrange[0:3000]
set xtics 200
set key bottom right

plot "../data/1000precision/no-async.dat" using 1:2 with linespoints ls 1 lc rgb "red" title "No asynchrony", \
	"../data/1000precision/uniform-async.dat" using 1:2 with linespoints ls 2 lc rgb "blue" title "Uniform asynchrony", \
	"../data/1000precision/normal-async.dat" using 1:2 with linespoints ls 3 lc rgb "green" title "Normal asynchrony", \
	"../data/1000precision/weibull-async.dat" using 1:2 with linespoints ls 4 lc rgb "orange" title "Weibull asynchrony"


!epstopdf "../graphs/async1000precision.eps"
!rm "../graphs/async1000precision.eps"
