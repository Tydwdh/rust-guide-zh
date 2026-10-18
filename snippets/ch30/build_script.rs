// build script：编译本包之前由 Cargo 编译并执行的程序。
// 通信方式是往 stdout 打 `cargo::` 开头的指令行（旧写法 `cargo:` 仍然可用）。
//
// 本文件与探针工作区里的 imgfmt/build.rs 内容一致，实测跑过。

fn main() {
    // 只在这几个输入变化时重跑；不写这行，包内任何文件变化都会触发重跑。
    println!("cargo::rerun-if-changed=build.rs");

    // 先声明「会有这个 cfg」，否则用它的代码会触发 unexpected_cfgs lint。
    println!("cargo::rustc-check-cfg=cfg(imgfmt_gpu)");
    println!("cargo::rustc-cfg=imgfmt_gpu");

    // 把目标三元组写进编译期环境变量，代码里用 env!("IMGFMT_TARGET") 读。
    let target = std::env::var("TARGET").unwrap();
    println!("cargo::rustc-env=IMGFMT_TARGET={target}");

    // 本地包的脚本警告默认就会打出来，形如 `warning: imgfmt@0.1.0: …`；
    // 被 Cargo 藏起来的只是脚本的裸 stdout —— 想看它得用 cargo build -vv。
    println!("cargo::warning=imgfmt build.rs ran, target = {target}");
}
