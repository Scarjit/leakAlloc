#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <pthread.h>

#define ITERATIONS 1000000

static double now_sec(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec + ts.tv_nsec / 1e9;
}

static void bench(const char *name, void (*fn)(void)) {
    double start = now_sec();
    fn();
    double elapsed = now_sec() - start;
    printf("%-30s %.3f sec  (%8.0f ops/sec)\n", name, elapsed, ITERATIONS / elapsed);
}

// Small allocations (64 bytes)
static void small_alloc(void) {
    for (int i = 0; i < ITERATIONS; i++) {
        void *p = malloc(64);
        *(volatile char *)p = 1;
    }
}

// Medium allocations (1KB)
static void medium_alloc(void) {
    for (int i = 0; i < ITERATIONS; i++) {
        void *p = malloc(1024);
        *(volatile char *)p = 1;
    }
}

// Large allocations (64KB)
static void large_alloc(void) {
    for (int i = 0; i < ITERATIONS / 10; i++) {
        void *p = malloc(64 * 1024);
        *(volatile char *)p = 1;
    }
}

// malloc + free cycle
static void malloc_free(void) {
    for (int i = 0; i < ITERATIONS; i++) {
        void *p = malloc(64);
        *(volatile char *)p = 1;
        free(p);
    }
}

// calloc (zeroed memory)
static void calloc_bench(void) {
    for (int i = 0; i < ITERATIONS; i++) {
        void *p = calloc(16, sizeof(int));
        *(volatile char *)p;
    }
}

// realloc growing
static void realloc_grow(void) {
    for (int i = 0; i < ITERATIONS / 100; i++) {
        void *p = malloc(64);
        for (int j = 0; j < 100; j++) {
            p = realloc(p, 64 + j * 16);
            *(volatile char *)p = 1;
        }
    }
}

// Varied sizes (16-4096 bytes)
static void varied_sizes(void) {
    for (int i = 0; i < ITERATIONS; i++) {
        size_t size = 16 + (i % 4096);
        void *p = malloc(size);
        *(volatile char *)p = 1;
    }
}

// Burst: allocate many, then free all
static void burst_alloc(void) {
    void *ptrs[1000];
    for (int i = 0; i < ITERATIONS / 1000; i++) {
        for (int j = 0; j < 1000; j++) {
            ptrs[j] = malloc(64);
            *(volatile char *)ptrs[j] = 1;
        }
        for (int j = 0; j < 1000; j++) {
            free(ptrs[j]);
        }
    }
}

// Fragmentation: interleaved alloc/free
static void fragmentation(void) {
    void *ptrs[100] = {0};
    for (int i = 0; i < ITERATIONS; i++) {
        int idx = i % 100;
        free(ptrs[idx]);
        ptrs[idx] = malloc(16 + (i % 512));
        *(volatile char *)ptrs[idx] = 1;
    }
}

// Thread worker
static void *thread_worker(void *arg) {
    (void)arg;
    for (int i = 0; i < ITERATIONS / 4; i++) {
        void *p = malloc(64);
        *(volatile char *)p = 1;
    }
    return NULL;
}

// Multi-threaded (4 threads)
static void multithread(void) {
    pthread_t threads[4];
    for (int i = 0; i < 4; i++) {
        pthread_create(&threads[i], NULL, thread_worker, NULL);
    }
    for (int i = 0; i < 4; i++) {
        pthread_join(threads[i], NULL);
    }
}

int main() {
    printf("%-30s %-10s %s\n", "Benchmark", "Time", "Throughput");
    printf("------------------------------------------------------\n");

    bench("small (64B)", small_alloc);
    bench("medium (1KB)", medium_alloc);
    bench("large (64KB) /10", large_alloc);
    bench("malloc+free", malloc_free);
    bench("calloc", calloc_bench);
    bench("realloc grow /100", realloc_grow);
    bench("varied (16-4096B)", varied_sizes);
    bench("burst (1000 alloc/free)", burst_alloc);
    bench("fragmentation", fragmentation);
    bench("multithread (4 threads)", multithread);

    return 0;
}
