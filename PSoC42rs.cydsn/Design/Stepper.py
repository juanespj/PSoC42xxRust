# /// script
# requires-python = ">=3.13"
# dependencies = [
#     "marimo>=0.19.0",
#     "pyzmq>=27.1.0",
# ]
# ///

import marimo

__generated_with = "0.19.7"
app = marimo.App()


@app.cell
def _():
    deg_per_rev=1.8
    steps_per_rev=360/deg_per_rev #steps
    microstepping=4
    target_spd=300#rpm
    target_rps=target_spd/60*microstepping
    print("Target RPS: ",   target_rps, "microsteps: ",microstepping)
    return microstepping, steps_per_rev


@app.cell
def _(microstepping, steps_per_rev):
    Pulser_clk=24e6
    pulser_per=1000
    pulserfreq=Pulser_clk/pulser_per
    print("Pulser Frequency: ", pulserfreq)
    pulser_maxspd= pulserfreq/steps_per_rev/microstepping
    print("Pulser Max Speed (RPS): ", pulser_maxspd)
    pulser_interval_us=1/pulserfreq*1e6
    print("Pulser Interval (us): ", pulser_interval_us)


    return


@app.cell
def _():
    return


@app.cell
def _():
    return


if __name__ == "__main__":
    app.run()
