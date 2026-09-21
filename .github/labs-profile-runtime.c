/* CI measurement only; never linked into a distributed runner.
 * https://clang.llvm.org/docs/SourceBasedCodeCoverage.html#using-the-profiling-runtime-without-static-initializers
 * Suppress LLVM's automatic initializer, which would add an environment entry
 * and an exit-time file writer even inside the worker's cleared environment.
 * The same binary runs every role. Only the trusted controller receives an
 * explicit profile path; workers never initialize/export their counters.
 */
#include <stdlib.h>

int __llvm_profile_runtime;
extern void __llvm_profile_initialize_file(void);
extern int __llvm_profile_write_file(void);

static void write_host_profile(void) { (void)__llvm_profile_write_file(); }

__attribute__((constructor)) static void initialize_host_profile(void) {
    const char *path = getenv("LLVM_PROFILE_FILE");
    if (path != NULL && path[0] != '\0') {
        __llvm_profile_initialize_file();
        if (atexit(write_host_profile) != 0) {
            abort();
        }
    }
}
