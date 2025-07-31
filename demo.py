#!/bin/env python3
# Simple demo script to call otdrs, parse out a few variables, and draw a data point graph with the front panel marked as a line with everything scaled appropriately
# Assumes metres are the units used in the SOR file.
import numpy as np

# You can use CBOR instead of JSON by swapping this and adding the --format cbor option below
# import cbor
import json
import subprocess
import otdrs
import matplotlib.pyplot as plt

sor = otdrs.parse_file("data/example1-noyes-ofl280.sor")

if (
    sor.fixed_parameters is None
    or sor.data_points is None
    or sor.general_parameters is None
    or sor.key_events is None
):
    raise RuntimeError("Missing some key blocks in this file!")

# To properly scale distances, we need to calculate the speed of light in fibre. This varies from fibre to fibre, but is about 1.47ish.
speed_of_light = 299792458.0  # m/s
# To accommodate this variation, the group index may be stored in the data (though if this is 0 you should use 146800)
refractive_index = sor.fixed_parameters.group_index / 100000.0
speed_of_light_in_fibre = (
    speed_of_light / refractive_index
)  # Calculate our speed of light for the reported group index/refractive index

# The following assumes only one pulse width used - this is true for almost all OTDR traces.
# Even acquisition regimes such as EXFO iOLM don't use the multiple pulse width/spacing convention, and just store data in proprietary blobs.
seconds_per_10k_points = (
    sor.fixed_parameters.data_spacing[0] / 1e10
)  # convert from picoseconds up
metres_per_data_spacing = (
    seconds_per_10k_points / 10000.0
) * speed_of_light_in_fibre  # FIXME: actually check this maths is right!
# Assumes only one scale factor used - same as above, this is generally "safe"!
sf = sor.data_points.scale_factors[0].scale_factor  # multiplier for the data
scaled_data = -(65535 - np.array(sor.data_points.scale_factors[0].data)) / float(
    sf
)  # Apply the scale factor to the whole dataset
# Just for cosmetics (in this example) we'll draw lines at the front panel point and the launch connector point
seconds_to_front_panel = sor.fixed_parameters.front_panel_offset / 1e10
seconds_to_launch_connector = sor.general_parameters.user_offset / 1e10
# And in metres, that's distance = time * speed
metres_to_front_panel = seconds_to_front_panel * speed_of_light_in_fibre
# Same again for launch - but we do need to offset from the front panel...
metres_to_launch_connector = (
    seconds_to_launch_connector * speed_of_light_in_fibre
) + metres_to_front_panel


# Let's print out our key events - helper function to avoid duplication for last_key_event...
def print_key_event(ke, sf, sol, fpo):
    loss = ke.event_loss / sf
    reflectance = ke.event_reflectance / sf
    seconds_to_event = ke.event_propogation_time / 1e10
    metres_to_event = (seconds_to_event * sol) + fpo
    print(
        "Event {}: {}dB loss, {}dB reflectance, {}m".format(
            ke.event_number, loss, reflectance, metres_to_event
        )
    )


# Now print the lot
for ke in sor.key_events.key_events:
    print_key_event(ke, sf, speed_of_light_in_fibre, metres_to_front_panel)
print_key_event(
    sor.key_events.last_key_event, sf, speed_of_light_in_fibre, metres_to_front_panel
)

print("E2E loss {}dB".format(sor.key_events.last_key_event.end_to_end_loss / sf))

# Plot our data - we calculate our X axis values based on the spacing interval and number of points to plot, on our scale of metres
# Note that "0" will be from the first data point, behind the front panel - normally we'd want to offset everything
# so that the 0 point is the launch connector, since that makes our subsequent event data make sense!
spacing = np.arange(
    0,
    metres_per_data_spacing * sor.data_points.scale_factors[0].n_points,
    metres_per_data_spacing,
)[0 : sor.data_points.scale_factors[0].n_points]
plt.plot(
    spacing,
    scaled_data,
    linewidth=1,
)
plt.axvline(x=metres_to_front_panel)  # draw a line at the front panel
plt.axvline(x=metres_to_launch_connector)  # draw a line at the front panel
plt.xlabel("Metres from OTDR module (not front panel/launch)")
plt.ylabel("dB")

plt.gca().invert_yaxis()
plt.show()
