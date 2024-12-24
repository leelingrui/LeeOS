define debug_kernel
target remote localhost:1234
file ./build/x86_64-unknown-none/debug/lee_os
end

define debug_init
target remote localhost:1234
file ./build/x86_64-unknown-leeos/debug/init
end


