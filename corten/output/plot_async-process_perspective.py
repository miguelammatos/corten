import sys

import utils

if __name__ == "__main__":
	if len(sys.argv) < 2:
		id = 1
	else:
		id = int(sys.argv[1])

	dirname = "async-plot/data/"
	indir = "original/"
	outdir = "cdf-process_perspective/"
	for name in ["no-async", "uniform-async", "normal-async", "weibull-async"]:
		filename = dirname + indir + name + ".dat"
		with open(filename, 'r') as f:
			constant_lst = [int(line.split()[0]) for line in f if int(line.split()[1]) == id]

			values, freq, freqsNormalized = utils.computeCDF(constant_lst, 10)

			data = [values, freqsNormalized]
			out_filename = dirname + outdir + name + str(id) + ".dat"
			caption = "Asynchrony"
			utils.dumpAsGnuplot(data, out_filename, caption, False)
