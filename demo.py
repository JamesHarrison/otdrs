#!/bin/env python3
# Simple demo script to call otdrs, parse out a few variables, and draw a data point graph with the front panel marked as a line with everything scaled appropriately
import numpy as np
import json
import subprocess
import matplotlib.pyplot as plt
# This works on Windows - modify path to suit, or work on indirectly generated data
proc = subprocess.Popen(["target/debug/otdrs.exe", "data/example1-noyes-ofl280.sor"], stdout=subprocess.PIPE)
out = proc.communicate()[0]
# Parse the data - we could load direct with pandas but want to be more prescriptive
otdrs_out = json.loads(out)
# some distance scaling stuff
speed_of_light = 299792458.0 # m/s
refractive_index = otdrs_out['fixed_parameters']['group_index'] / 100000.0 # convert refractice index accordingly
speed_of_light_in_fibre = speed_of_light / refractive_index # adjust our speed of light for the reported group index/refractive index
# Assumes only one pulse width used
seconds_per_10k_points = (otdrs_out['fixed_parameters']['data_spacing'][0]/1e10) # convert from picoseconds up
metres_per_data_spacing = (((seconds_per_10k_points/10000.0)) * speed_of_light_in_fibre) # todo: actually check this maths is right
# Assumes only one scale factor used
sf = otdrs_out['data_points']['scale_factors'][0]['scale_factor'] # multiplier for the data
scaled_data = np.array(otdrs_out['data_points']['scale_factors'][0]['data'])/float(sf) # apply the scale factor
seconds_to_front_panel = otdrs_out['fixed_parameters']['front_panel_offset']/1e10
metres_to_front_panel = (((seconds_to_front_panel)) * speed_of_light_in_fibre)

spacing = np.arange(0, metres_per_data_spacing*otdrs_out['data_points']['scale_factors'][0]['n_points'], metres_per_data_spacing)[0:otdrs_out['data_points']['scale_factors'][0]['n_points']]
plt.plot(spacing, scaled_data, linewidth=1, )
plt.axvline(x=metres_to_front_panel) # draw a line at the front panel
plt.xlabel("Metres")
plt.ylabel("dB")

plt.gca().invert_yaxis()
plt.show()