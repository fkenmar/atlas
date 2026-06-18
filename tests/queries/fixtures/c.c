/* Fixture for queries/c/tags.scm — one construct per extraction rule.
   Not compiled by cargo; it only needs to parse. */

#include <stdio.h>
#include "config.h"

struct Point {
    int x;
    int y;
};

enum Color {
    RED,
    GREEN,
    BLUE
};

typedef struct Point PointAlias;

/* A prototype-only declaration (as found in a header). */
void reset(void);

int add(int a, int b) {
    return a + b;
}

static int helper(int x) {
    return add(x, 1);
}
