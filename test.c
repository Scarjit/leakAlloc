#include <stdio.h>
#include <stdlib.h>
#include <string.h>

void *memalign(size_t alignment, size_t size);

int main() {
    // Test malloc
    char *str = malloc(64);
    strcpy(str, "Hello from leak allocator!");
    printf("malloc: %s\n", str);

    // Test calloc
    int *arr = calloc(10, sizeof(int));
    printf("calloc: arr[0]=%d arr[9]=%d (should be 0)\n", arr[0], arr[9]);

    // Test realloc
    str = realloc(str, 128);
    strcat(str, " - extended!");
    printf("realloc: %s\n", str);

    // Test free (should be no-op)
    free(str);
    free(arr);
    printf("free: completed (no-op)\n");

    // Test memalign
    void *aligned = memalign(4096, 128);
    printf("memalign: ptr=%p aligned=%s\n", aligned,
           ((size_t)aligned % 4096) == 0 ? "YES" : "NO");

    void *aligned2 = memalign(64, 256);
    printf("memalign: ptr=%p aligned=%s\n", aligned2,
           ((size_t)aligned2 % 64) == 0 ? "YES" : "NO");

    // Stress test
    for (int i = 0; i < 1000; i++) {
        void *p = malloc(1024);
        if (!p) {
            printf("OOM at iteration %d\n", i);
            return 1;
        }
    }
    printf("stress: 1000 x 1KB allocations OK\n");

    return 0;
}
