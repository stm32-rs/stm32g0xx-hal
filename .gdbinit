target remote :3333

# monitor itm port 0 on
# monitor tpiu config internal /tmp/itm.fifo uart off 2000000
monitor arm semihosting enable
load
continue