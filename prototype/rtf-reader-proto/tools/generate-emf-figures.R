#!/usr/bin/env Rscript
# Generate a set of representative clinical figures as EMF files.
# Output: test-data/figures/figure-01.emf ... figure-10.emf

library(devEMF)

out_dir <- "test-data/figures"
dir.create(out_dir, showWarnings = FALSE, recursive = TRUE)

set.seed(42)

fig_specs <- list(
  list(name = "scatter", title = "Figure 1: Scatter Plot of Biomarker vs Response", fn = function() {
    x <- rnorm(100)
    y <- 2 * x + rnorm(100)
    plot(x, y, pch = 19, col = "steelblue", main = "Biomarker vs Response",
         xlab = "Biomarker", ylab = "Response")
    abline(lm(y ~ x), col = "red", lwd = 2)
  }),
  list(name = "barplot", title = "Figure 2: Treatment Response by Arm", fn = function() {
    arms <- c("Placebo", "Drug 10 mg", "Drug 20 mg")
    resp <- c(12, 28, 35)
    barplot(resp, names.arg = arms, col = c("gray", "skyblue", "darkblue"),
            main = "Treatment Response", ylab = "Responder Count")
  }),
  list(name = "boxplot", title = "Figure 3: Age Distribution by Sex", fn = function() {
    ages <- list(Male = rnorm(80, 55, 12), Female = rnorm(80, 58, 11))
    boxplot(ages, col = c("lightblue", "pink"), main = "Age Distribution by Sex")
  }),
  list(name = "histogram", title = "Figure 4: Distribution of Lab Values", fn = function() {
    vals <- rnorm(500, 100, 15)
    hist(vals, breaks = 30, col = "lightgreen", main = "Lab Value Distribution",
         xlab = "Value")
  }),
  list(name = "kmcurve", title = "Figure 5: Kaplan-Meier Survival Curve", fn = function() {
    t <- 1:24
    s1 <- exp(-t / 20)
    s2 <- exp(-t / 12)
    plot(t, s1, type = "l", col = "blue", ylim = c(0, 1), xlab = "Month",
         ylab = "Survival Probability", main = "Kaplan-Meier Curve")
    lines(t, s2, col = "red")
    legend("topright", legend = c("Arm A", "Arm B"), col = c("blue", "red"), lty = 1)
  }),
  list(name = "forest", title = "Figure 6: Forest Plot of Hazard Ratios", fn = function() {
    or <- c(0.8, 1.2, 0.6, 1.5)
    lower <- or - 0.2
    upper <- or + 0.3
    labels <- c("Age", "Sex", "Race", "Arm")
    plot(NA, xlim = c(0.2, 2.0), ylim = c(0.5, 4.5), xlab = "Odds Ratio",
         ylab = "", yaxt = "n", main = "Forest Plot")
    abline(v = 1, lty = 2, col = "gray")
    axis(2, at = 1:4, labels = labels, las = 2)
    arrows(lower, 1:4, upper, 1:4, code = 3, angle = 90, length = 0.05, col = "black")
    points(or, 1:4, pch = 15, col = "darkred")
  }),
  list(name = "line", title = "Figure 7: Longitudinal Lab Values", fn = function() {
    weeks <- 0:12
    v1 <- 100 + 2 * weeks + rnorm(13, 0, 3)
    v2 <- 100 - 1 * weeks + rnorm(13, 0, 3)
    plot(weeks, v1, type = "l", col = "blue", ylim = c(90, 130), xlab = "Week",
         ylab = "Lab Value", main = "Longitudinal Lab Values")
    lines(weeks, v2, col = "red")
    legend("topright", legend = c("Arm A", "Arm B"), col = c("blue", "red"), lty = 1)
  }),
  list(name = "pie", title = "Figure 8: Disposition of Subjects", fn = function() {
    vals <- c(80, 15, 5)
    labels <- c("Completed", "Discontinued", "Ongoing")
    pie(vals, labels = labels, col = c("green", "orange", "blue"),
        main = "Subject Disposition")
  }),
  list(name = "density", title = "Figure 9: Density Plot of Response", fn = function() {
    d1 <- density(rnorm(200, 50, 10))
    d2 <- density(rnorm(200, 60, 12))
    plot(d1, col = "blue", main = "Response Density", xlab = "Response")
    lines(d2, col = "red")
    legend("topright", legend = c("Arm A", "Arm B"), col = c("blue", "red"), lty = 1)
  }),
  list(name = "corrmatrix", title = "Figure 10: Correlation Heatmap", fn = function() {
    m <- matrix(c(1, 0.6, 0.3, 0.6, 1, 0.5, 0.3, 0.5, 1), nrow = 3)
    image(m, col = hcl.colors(20, "Blues"), main = "Correlation Matrix",
          axes = FALSE)
    axis(1, at = c(0, 0.5, 1), labels = c("A", "B", "C"))
    axis(2, at = c(0, 0.5, 1), labels = c("A", "B", "C"))
  })
)

for (i in seq_along(fig_specs)) {
  spec <- fig_specs[[i]]
  file <- file.path(out_dir, sprintf("figure-%02d.emf", i))
  emf(file, width = 6, height = 4)
  spec$fn()
  dev.off()
  message(sprintf("Generated: %s (%s)", file, spec$name))
}

message("All figures generated in ", out_dir)
