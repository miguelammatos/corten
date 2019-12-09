set term postscript color eps enhanced 22
set output '../graphs/async-process_perspective.eps'
#load "styles.inc"
set size 1.4,1

set xlabel "Time (time units)"
set ylabel "CDF \nof local method calls"
set yrange[0.0:1.0]
#set ytics 100	
#set mytics 5
#set xrange[0:3000]
#set xtics 200
set key bottom right

#set title "From the perspective of process 1"
#show title

if (!exists("id")) id=1

plot "../data/cdf-process_perspective/no-async".id.".dat" using 1:2 with linespoints ls 1 lc rgb "red" title "No asynchrony", \
	"../data/cdf-process_perspective/uniform-async".id.".dat" using 1:2 with linespoints ls 2 lc rgb "blue" title "Uniform asynchrony", \
	"../data/cdf-process_perspective/normal-async".id.".dat" using 1:2 with linespoints ls 3 lc rgb "green" title "Normal asynchrony", \
	"../data/cdf-process_perspective/weibull-async".id.".dat" using 1:2 with linespoints ls 4 lc rgb "orange" title "Weibull asynchrony"


!epstopdf "../graphs/async-process_perspective.eps"
!rm "../graphs/async-process_perspective.eps"
