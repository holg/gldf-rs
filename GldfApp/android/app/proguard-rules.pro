# Keep UniFFI generated code
-keep class uniffi.** { *; }
-keepclassmembers class uniffi.** { *; }

# Keep JNA
-keep class com.sun.jna.** { *; }
-keepclassmembers class com.sun.jna.** { *; }

# Keep native methods
-keepclasseswithmembernames class * {
    native <methods>;
}
