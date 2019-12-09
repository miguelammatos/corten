import os
import utils

if __name__ == "__main__":
	dirname = "async-plot/data/"
	indir = "original/"
	outdir = "values-cdf/"

	if not os.path.exists(dirname + outdir):
		os.makedirs(dirname + outdir)

	for name in ["no-async", "uniform-async", "normal-async", "weibull-async"]:
		filename = dirname + indir + name + ".dat"
		with open(filename, 'r') as f:
			constant_lst = [int(line.split()[0]) for line in f]

			values, freq, freqsNormalized = utils.computeCDF(constant_lst, 10)

			data = [values, freqsNormalized]
			out_filename = dirname + outdir + name + ".dat"
			caption = "Asynchrony"
			utils.dumpAsGnuplot(data, out_filename, caption, False)
