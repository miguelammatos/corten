import sys
import ast

import utils

def line_to_num_of_processes(line): 
	l = line.split()
	return int(l[2])*int(l[3])

def cumulative(lst):
	for i in range(1, len(lst)):
		lst[i] += lst[i-1]

if __name__ == "__main__":
	if len(sys.argv) < 2:
		id = 9
	else:
		id = int(sys.argv[1])

	with open("churn-plot/churn.dat", 'r') as f:
		values = [(int(line.split()[0]), int(line.split()[2])) for line in f if id in ast.literal_eval(line.split()[4])]
		
		data = map(list, zip(*values))

		cumulative(data[1])

		out_filename = "churn-plot/churn-transformed-process_perspective.dat"
		caption = "Churn"
		utils.dumpAsGnuplot(data, out_filename, caption, False)


