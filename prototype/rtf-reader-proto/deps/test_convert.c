#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "libemf2svg/inc/emf2svg.h"

int main(int argc, char **argv) {
    if (argc < 2) {
        fprintf(stderr, "Usage: %s <emf-file>\n", argv[0]);
        return 1;
    }

    FILE *f = fopen(argv[1], "rb");
    if (!f) {
        perror("fopen");
        return 1;
    }
    fseek(f, 0, SEEK_END);
    long size = ftell(f);
    fseek(f, 0, SEEK_SET);
    char *data = malloc(size);
    fread(data, 1, size, f);
    fclose(f);

    generatorOptions options = {0};
    options.verbose = true;
    options.emfplus = true;
    options.svgDelimiter = true;
    options.imgWidth = 0;
    options.imgHeight = 0;

    char *out = NULL;
    size_t out_len = 0;
    int ret = emf2svg(data, size, &out, &out_len, &options);
    printf("ret=%d out_len=%zu\n", ret, out_len);
    if (ret == 0 && out) {
        printf("%.*s\n", (int)out_len, out);
        free(out);
    }
    free(data);
    return ret;
}
