# configuration file generated by Bochs
plugin_ctrl: unmapped=true, biosdev=true, speaker=true, extfpuirq=true, parallel=true, serial=true, iodebug=true
config_interface: textconfig
display_library: x
memory: host=32M, guest=32M
romimage: file="/usr/local/share/bochs/BIOS-bochs-latest", address=0x00000000, options=none
vgaromimage: file="/usr/local/share/bochs/VGABIOS-lgpl-latest"
boot: disk
floppy_bootsig_check: disabled=0
floppya: type=1_44
# no floppyb
ata0: enabled=true, ioaddr1=0x1f0, ioaddr2=0x3f0, irq=14
ata0-master: type=disk, path=build/master.img, mode=flat
ata0-slave: type=disk, path=build/slave.img, mode=flat
ata1: enabled=true, ioaddr1=0x170, ioaddr2=0x370, irq=15
ata1-master: type=none
ata1-slave: type=none
ata2: enabled=false
ata3: enabled=false
optromimage1: file=none
optromimage2: file=none
optromimage3: file=none
optromimage4: file=none
optramimage1: file=none
optramimage2: file=none
optramimage3: file=none
optramimage4: file=none
pci: enabled=1, chipset=i440fx, slot1=none, slot2=none, slot3=none, slot4=none, slot5=none
vga: extension=vbe, update_freq=5, realtime=1, ddc=builtin
cpu: count=1:1:1, ips=4000000, model=bx_generic, reset_on_triple_fault=1, cpuid_limit_winnt=0, ignore_bad_msrs=1, mwait_is_nop=0
cpuid: x86_64=1, level=6, stepping=3, model=3, family=6, vendor_string="GenuineIntel", brand_string="              Intel(R) Pentium(R) 4 CPU        "
cpuid: mmx=true, apic=xapic, simd=sse2, sse4a=false, misaligned_sse=false, sep=true
cpuid: movbe=false, adx=false, aes=false, sha=false, xsave=false, xsaveopt=false, smep=false
cpuid: smap=false, mwait=true
print_timestamps: enabled=0
debugger_log: -
gdbstub: enabled=1, port=1234, text_base=0, data_base=0, bss_base=0
port_e9_hack: enabled=0
private_colormap: enabled=0
clock: sync=none, time0=local, rtc_sync=0
# no cmosimage
log: -
logprefix: %t%e%d
debug: action=ignore
info: action=report
error: action=report
panic: action=ask
keyboard: type=mf, serial_delay=250, paste_delay=100000, user_shortcut=none
mouse: type=ps2, enabled=false, toggle=ctrl+mbutton
speaker: enabled=true, mode=system
parport1: enabled=true, file=none
parport2: enabled=false
com1: enabled=true, mode=null
com2: enabled=false
com3: enabled=false
com4: enabled=false
