import utils


def parse(f): 
	d = {}
	for line in f:
		time = int(line.split()[0])
		id = int(line.split()[1])
		if id in d:
			d[id].append(time)
		else:
			d[id] = [time]
	for k in d:
		for i in range(len(d[k])-1, 0, -1):
			d[k][i] -= d[k][i-1]
	return [item for sublist in d.values() for item in sublist]

if __name__ == "__main__":
	dirname = "async-plot/data/"
	indir = "original/"
	outdir = "delta/"
	for name in ["no-async", "uniform-async", "normal-async", "weibull-async"]:
		filename = dirname + indir + name + ".dat"
		with open(filename, 'r') as f:
			lst = parse(f)
			
			values, freq, freqsNormalized = utils.computeCDF(lst, 10)

			data = [values, freqsNormalized]
			out_filename = dirname + outdir + name + ".dat"
			caption = "Asynchrony"
			utils.dumpAsGnuplot(data, out_filename, caption, False)
