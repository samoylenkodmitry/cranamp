//! Apply the rendered background's contrast to Android's system bar icons.
use jni::{
    jni_sig, jni_str,
    objects::{Global, JClass},
    EnvUnowned, JavaVM,
};
use std::sync::OnceLock;

struct Bridge {
    vm: JavaVM,
    class: Global<JClass<'static>>,
}
static BRIDGE: OnceLock<Bridge> = OnceLock::new();

// This is the sole JNI export for CranampApplication.nativeInitialize. The
// implementation uses scoped JNI references and contains no raw pointer access.
#[expect(
    unsafe_code,
    reason = "JNI resolves this application-owned export by its symbol name"
)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_cranamp_app_CranampApplication_nativeInitialize<'local>(
    mut env: EnvUnowned<'local>,
    class: JClass<'local>,
) {
    let _ = env.with_env(|env| -> jni::errors::Result<()> {
        let _ = BRIDGE.set(Bridge {
            vm: env.get_java_vm()?,
            class: env.new_global_ref(&class)?,
        });
        Ok(())
    });
}

pub(crate) fn set_dark_system_bar_icons(dark: bool) {
    let Some(bridge) = BRIDGE.get() else {
        return;
    };
    if let Err(error) = bridge
        .vm
        .attach_current_thread(|env| -> jni::errors::Result<()> {
            let result = env.call_static_method(
                &bridge.class,
                jni_str!("setDarkSystemBarIcons"),
                jni_sig!("(Z)V"),
                &[dark.into()],
            );
            if result.is_err() {
                env.exception_clear();
            }
            result.map(|_| ())
        })
    {
        log::warn!("Could not update Android system bar contrast: {error}");
    }
}
