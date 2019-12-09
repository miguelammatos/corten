set term postscript color eps enhanced 22
set output 'churn-process_perspective.eps'
#load "styles.inc"
set size 1,0.6 



set xlabel "Time (time units)"
set ylabel "State"
set yrange[0:1.5]
set ytics 1	
#set mytics 5
set xrange[0:650]
set xtics 100
set key top left 

#####
# https://stackoverflow.com/questions/35085610/gnuplot-how-to-add-a-non-number-value-on-y-axis
#Fval=-1
#set ytics 0,0.1,1; 
set ytics add ("online" 1);
set ytics add ("offline" 0);
#####

plot "churn-transformed-process_perspective.dat" using 1:2 with steps ls 1 lc rgb "red" title "Churn"

!epstopdf "churn-process_perspective.eps"
!rm "churn-process_perspective.eps"
