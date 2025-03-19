build_win:
  cargo build --target=x86_64-pc-windows-gnu --release
  wine ~/SDK/rcedit/rcedit-x64.exe /Users/allen/Documents/RustProjects/PngOptimizier/target/x86_64-pc-windows-gnu/release/png_optimizer.exe --set-icon /Users/allen/Documents/RustProjects/PngOptimizier/asset/icon.ico
