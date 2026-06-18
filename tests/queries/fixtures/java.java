// Fixture for queries/java/tags.scm — one construct per extraction rule.
// Not compiled by cargo; it only needs to parse.

package com.example.service;

import java.util.List;
import java.util.Map;

public class Service {
    public static final String API_VERSION = "1.0";
    private int count;

    public Service(int count) {
        this.count = count;
    }

    public int total() {
        return helper(count);
    }

    private int helper(int x) {
        return x + 1;
    }
}

interface Runner {
    void run();
}

enum Level {
    LOW,
    HIGH
}
