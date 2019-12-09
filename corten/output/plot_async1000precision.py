import utils

if __name__ == "__main__":
	dirname = "async-plot/data/"
	indir = "original/"
	outdir = "1000precision/"
	for name in ["no-async", "uniform-async", "normal-async", "weibull-async"]:
		filename = dirname + indir + name + ".dat"
		with open(filename, 'r') as f:
			constant_lst = [int(line.split()[0]) for line in f]

			values, freq, freqsNormalized = utils.computeCDF(constant_lst)

			data = [values, freqsNormalized]
			out_filename = dirname + outdir + name + ".dat"
			caption = "Asynchrony"
			utils.dumpAsGnuplot(data, out_filename, caption, False)
