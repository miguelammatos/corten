set term postscript color eps enhanced 22
set output 'churn.eps'
#load "styles.inc"
set size 1,0.6 

set xlabel "Time (time units)"
set ylabel "Number of processes up"
set yrange[0:12]
#set ytics 100	
#set mytics 5
set xrange[0:650]
set xtics 50
set key bottom right 

plot "churn-transformed.dat" using 1:2 with steps ls 1 lc rgb "red" title "Churn"

!epstopdf "churn.eps"
!rm "churn.eps"
