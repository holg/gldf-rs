//! Milestone M4: this compiles + runs in a native window with working clicks.
//!
//! Until the tachys fork exists this file is aspirational — it defines the
//! acceptance test for the spike.

fn main() {
    // With the patched fork:
    //
    // use leptos::prelude::*;
    //
    // fn app() -> impl IntoView {
    //     let count = RwSignal::new(0);
    //     view! {
    //         <div style="padding: 2rem; font-family: sans-serif;">
    //             <h1>"leptos-blitz"</h1>
    //             <button on:click=move |_| *count.write() += 1>
    //                 "Count: " {count}
    //             </button>
    //         </div>
    //     }
    // }
    //
    // leptos_blitz::launch(app);
    eprintln!("Apply docs/PATCH_PLAN.md to your leptos fork first, then enable the code above.");
}
