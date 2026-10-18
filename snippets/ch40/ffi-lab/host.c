/* host.c —— C 侧宿主，链接到 cargo 编出来的 pointops.dll。
 * 编译：gcc host.c -o host.exe -L target/release -lpointops
 * 运行前把 target/release 里的 pointops.dll 放到 host.exe 旁边（或加进 PATH）。 */
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

/* 与 Rust 侧 #[repr(C)] struct Summary 一一对应：字段、顺序、类型都相同。 */
typedef struct {
    uint64_t sum;
    uint64_t count;
} Summary;

/* 不透明句柄：C 侧只知道它是个指针，完全不知道 Counter 长什么样。 */
void *counter_new(void);
uint64_t counter_add(void *handle, uint64_t n);
void counter_free(void *handle);
Summary summarize(const uint32_t *values, size_t len);

int main(void) {
    void *c = counter_new();
    printf("add(3) = %llu\n", (unsigned long long)counter_add(c, 3));
    printf("add(4) = %llu\n", (unsigned long long)counter_add(c, 4));
    counter_free(c);

    uint32_t values[] = {1, 2, 3, 4};
    Summary s = summarize(values, sizeof(values) / sizeof(values[0]));
    printf("sum = %llu, count = %llu\n",
           (unsigned long long)s.sum, (unsigned long long)s.count);
    return 0;
}
