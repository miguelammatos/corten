import utils

def line_to_num_of_processes(line): 
	l = line.split()
	return int(l[2])*int(l[3])

def cumulative(lst):
	for i in range(1, len(lst)):
		lst[i] += lst[i-1]

if __name__ == "__main__":
	with open("churn-plot/churn.dat", 'r') as f:
		values = [(int(line.split()[0]), line_to_num_of_processes(line)) for line in f]
		
		data = map(list, zip(*values))

		cumulative(data[1])

		out_filename = "churn-plot/churn-transformed.dat"
		caption = "Churn"
		utils.dumpAsGnuplot(data, out_filename, caption, False)


