//! The crate's WESL validator: compiles every shader module to WGSL and parses the result with
//! naga, without a GPU and in a second.
//!
//! A WESL error is **silent at build time**: the shaders are embedded with `embedded_asset!`, so a
//! struct constructed with the wrong number of fields, a name used from another module without an
//! import, a local that shadows another local, or a `pow(x, p)` with `p` in a `let` all compile, are
//! embedded, and then fail in the GPU driver at run time. The symptom is a surface that renders as a
//! flat dark grey, and the only clue is a `failed to process shader` line in the run's log — which a
//! capture script throws away.
//!
//! ```sh
//! cargo run --release --manifest-path tools/wesl-check/Cargo.toml -- src
//! ```
//!
//! The path is the crate's `src` directory, where the modules live. Bevy's own shader modules are not
//! on disk, so they are stubbed; a stub only has to *exist* for the import to resolve.
//!
//! # Why the stubs are mounted the way they are
//!
//! A WESL import's path is not the text that was written. `import bevy_pbr::render::view;` inside a
//! module of the crate's own package arrives at the resolver as
//! `PathOrigin::Package("bevy_lights/bevy_pbr")` with components `["render", "view"]`: WESL joins the
//! import against the importing module's path, and a `Package` joined onto a `Package` takes the
//! sub-package form `parent/child`, a spelling no source file can write (see `join_path` in
//! `wgsl-parse`). A router dispatches on an *origin equality*, so the Bevy namespaces must each be
//! mounted twice — once as themselves and once as `bevy_lights/<them>` — and the virtual modules
//! inside each mount are keyed by `Absolute` paths, because that is the form the router hands on.

use std::borrow::Cow;
use std::path::Path;

use wesl::error::ResolveError;
use wesl::resolver::{FileResolver, Resolver, Router, VirtualResolver};
use wgsl_parse::syntax::ModulePath;

/// The crate's own modules. Every one of them: a WESL error is silent at build time, so a module
/// nobody compiles is a module that fails in the GPU driver.
const MODULES: &[&str] = &["bevy_lights::flame"];

/// The Bevy namespaces the crate's shaders import from. Each is mounted under its own name and under
/// the crate's sub-package spelling.
const BEVY_NAMESPACES: &[&str] = &["bevy_pbr", "bevy_render", "bevy_core_pipeline", "constants"];

/// Bevy's own shader modules, stubbed: they are compiled into Bevy and are not on disk, but the
/// crate's modules import them and the import has to resolve. Generated from the imports with
///
/// ```sh
/// grep -rh "^import bevy_" src/ --include=*.wesl | sed 's/^import //; s/::.*//'
/// ```
const BEVY_MODULES: &[&str] = &[
    "bevy_pbr::render::forward_io",
    "bevy_pbr::render::mesh_functions",
    "bevy_pbr::render::mesh_view_bindings",
    "bevy_pbr::render::view_transformations",
];

/// The errors that are not the crate's shaders' fault, and so are reported rather than counted as
/// failures: a Bevy module the stub list does not have. The stub list is a convenience for checking
/// the crate's own code, not a complete model of Bevy.
const BENIGN: &[&str] = &[
    "is declared with `package` visibility",
    "module not found",
];

/// The path to register a virtual module under: `Absolute` components, because that is the form the
/// router strips a mount's prefix down to.
fn virtual_path(name: &str) -> ModulePath {
    ModulePath::new(
        wgsl_parse::syntax::PathOrigin::Absolute,
        name.split("::").map(str::to_string).collect(),
    )
}

/// A stub's source: the declarations the crate's shaders reach for, with Bevy's own signatures.
///
/// A stub is not Bevy's implementation — it is the shape of the interface, so that a call with the
/// wrong arity, a field that does not exist or an unimported name fails here instead of in the GPU
/// driver. `constants` carries the bind-group numbers Bevy substitutes from Rust.
fn stub_source(name: &str) -> String {
    let source = match name {
        "constants" => {
            "const MATERIAL_BIND_GROUP: u32 = 1u;\nconst MESH_BIND_GROUP: u32 = 0u;\n"
        }
        // The children, re-exported so `import bevy_pbr::render::{forward_io, mesh_functions};`
        // resolves each as a namespace.
        "bevy_pbr::render" => {
            "import bevy_pbr::render::forward_io;\n\
             import bevy_pbr::render::mesh_functions;\n\
             import bevy_pbr::render::mesh_view_bindings;\n\
             import bevy_pbr::render::view_transformations;\n"
        }
        "bevy_pbr::render::forward_io" => {
            "struct Vertex {\n\
             \x20   @builtin(instance_index) instance_index: u32,\n\
             \x20   @location(0) position: vec3<f32>,\n\
             \x20   @location(2) uv: vec2<f32>,\n\
             };\n\
             struct UncompressedVertex {\n\
             \x20   instance_index: u32,\n\
             \x20   position: vec3<f32>,\n\
             \x20   uv: vec2<f32>,\n\
             };\n\
             fn decompress_vertex(vertex: Vertex, instance_index: u32) -> UncompressedVertex {\n\
             \x20   return UncompressedVertex(instance_index, vertex.position, vertex.uv);\n\
             }\n"
        }
        "bevy_pbr::render::mesh_functions" => {
            "fn get_world_from_local(instance_index: u32) -> mat4x4<f32> {\n\
             \x20   var identity = mat4x4<f32>();\n\
             \x20   identity[0][0] = 1.0;\n\
             \x20   identity[1][1] = 1.0;\n\
             \x20   identity[2][2] = 1.0;\n\
             \x20   identity[3][3] = 1.0;\n\
             \x20   return identity;\n\
             }\n\
             fn get_tag(instance_index: u32) -> u32 {\n\
             \x20   return 0u;\n\
             }\n"
        }
        "bevy_pbr::render::mesh_view_bindings" => {
            "struct View {\n\
             \x20   world_from_view: mat4x4<f32>,\n\
             \x20   clip_from_world: mat4x4<f32>,\n\
             };\n\
             @group(0) @binding(0) var<uniform> view: View;\n"
        }
        "bevy_pbr::render::view_transformations" => {
            "fn position_world_to_clip(world: vec3<f32>) -> vec4<f32> {\n\
             \x20   return vec4<f32>(world, 1.0);\n\
             }\n"
        }
        _ => "",
    };
    source.to_string()
}

/// A resolver that serves the crate's own `.wesl` files.
struct Files(FileResolver);

impl Resolver for Files {
    fn resolve_source<'a>(&'a self, path: &ModulePath) -> Result<Cow<'a, str>, ResolveError> {
        if path.is_root() {
            return Ok(String::new().into());
        }
        self.0.resolve_source(path)
    }
}

fn main() {
    let root = std::env::args().nth(1).unwrap_or_else(|| "src".to_string());
    let root = Path::new(&root);

    let mut files = FileResolver::new(root);
    files.set_extension("wesl");
    files.set_root(root);

    let mut router = Router::new();
    router.mount_resolver(
        "bevy_lights".parse().expect("the crate's name"),
        Files(files),
    );

    for namespace in BEVY_NAMESPACES {
        for spelling in [namespace.to_string(), format!("bevy_lights/{namespace}")] {
            let mut stubs = VirtualResolver::new();
            // Every WESL package has a root module (`package`), which is the empty path, not a
            // component called `package`; a mount that cannot answer it fails every import.
            stubs.add_module(ModulePath::new_root(), String::new().into());
            if *namespace == "constants" {
                // `constants::MATERIAL_BIND_GROUP` is the *root* of the `constants` package: the
                // module path is the mount itself, empty, not a component called `constants`.
                stubs.add_module(ModulePath::new_root(), stub_source("constants").into());
            } else if *namespace == "bevy_pbr" {
                // `bevy_pbr::render` exists for its children: Bevy's shaders import through it.
                stubs.add_module(virtual_path("render"), stub_source("bevy_pbr::render").into());
            }
            for name in BEVY_MODULES {
                if let Some(rest) = name.strip_prefix(&format!("{namespace}::")) {
                    stubs.add_module(virtual_path(rest), stub_source(name).into());
                }
            }
            router.mount_resolver(spelling.parse().expect("a namespace"), stubs);
        }
    }

    // `keep_main`: a library module has no entry point, and stripping keeps only what an entry point
    // reaches, so without it a library compiled to an *empty* string and "the WGSL is ok" checked
    // nothing past the parser (a `let x: f32 = vec3<f32>(1.0);` in a library module passed). Keeping
    // the module's own declarations pulls in what they import, and `visibility: false` is how Bevy
    // compiles the crate's shaders, so a `package` item imported across the crate's modules is not
    // refused here when Bevy accepts it.
    let options = wesl::CompileOptions {
        keep_main: true,
        visibility: false,
        ..Default::default()
    };
    let compiler = wesl::Compiler::new_with_resolver(options, router);

    let mut failed = 0;
    for module in MODULES {
        let path: ModulePath = match module.parse() {
            Ok(path) => path,
            Err(e) => {
                println!("{module}: the module path is wrong: {e}");
                failed += 1;
                continue;
            }
        };
        match compiler.compile_module(&path) {
            Ok(result) => {
                let wgsl = result.to_string();
                match naga::front::wgsl::parse_str(&wgsl) {
                    Ok(_) => println!("{module}: the WGSL is ok"),
                    Err(e) => {
                        println!("{module}: the WGSL has an error:\n{}", e.emit_to_string(&wgsl));
                        failed += 1;
                    }
                }
            }
            Err(e) => {
                let text = e.to_string();
                if let Some(benign) = BENIGN.iter().find(|b| text.contains(**b)) {
                    println!("{module}: not checked ({benign}): {text}");
                } else {
                    println!("{module}: ERROR {e}");
                    failed += 1;
                }
            }
        }
    }

    if failed > 0 {
        eprintln!("{failed} module(s) failed");
        std::process::exit(1);
    }
}
