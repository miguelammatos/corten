import utils

if __name__ == "__main__":
	dir_name = "network-plot/"
	for name in ["constant", "matrix"]:
		filename = dir_name + "latency-" + name + ".dat"

		with open(filename, 'r') as f:
			constant_lst = [int(line.strip()) for line in f]
			#print(constant_lst)

			values, freq, freqsNormalized = utils.computeCDF(constant_lst, 40)

			# print(values)
			# print()
			# print(freq)
			# print()
			# print(freqsNormalized)

			data = [values, freqsNormalized]
			out_filename = dir_name + "latency-" + name + "-cdf.dat"
			caption = name[0].upper() + name[1:] + " network latency"
			utils.dumpAsGnuplot(data, out_filename, caption, False)


