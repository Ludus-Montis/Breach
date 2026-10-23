class_name NativeLibrary
extends RefCounted
## Breach's native core (native/README.md), loaded once whoever needs it first: the content
## (CrateContent, which always does: the crate loads it, Decision 153) or the native kernels
## (NativeKernels, when BREACH_NATIVE=rust). native/ holds a .gdignore, so the editor never
## loads it on its own; native/build.sh builds it.

const EXTENSION := "res://native/breach_rust.gdextension"
const NAME := "breach_rust"


## True once the core's class `class_probe` exists, loading the library if it is built.
static func load_for(class_probe: String) -> bool:
	if not ClassDB.class_exists(class_probe) and FileAccess.file_exists(binary()):
		GDExtensionManager.load_extension(EXTENSION)
	return ClassDB.class_exists(class_probe)


## Where native/build.sh puts the library for this platform.
static func binary() -> String:
	match OS.get_name():
		"Windows":
			return "res://native/bin/%s.windows.x86_64.dll" % NAME
		"macOS":
			return "res://native/bin/lib%s.macos.universal.dylib" % NAME
	return "res://native/bin/lib%s.linux.x86_64.so" % NAME
