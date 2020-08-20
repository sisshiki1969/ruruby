# ruruby benchmark results

## environment

Ruby version: 2.8.0  
CPU: Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz  
OS: Ubuntu 20.04 LTS

## execution time

|     benchmark     |      ruby      |     ruruby     |  rate  |
| :---------------: | :------------: | :------------: | :----: |
|     block.rb      | 0.25 ± 0.01 s  | 0.48 ± 0.03 s  | x 1.94 |
|    for_loop.rb    | 0.32 ± 0.03 s  | 0.38 ± 0.00 s  | x 1.19 |
| so_mandelbrot.rb  | 2.04 ± 0.06 s  | 2.14 ± 0.08 s  | x 1.05 |
| app_mandelbrot.rb | 1.16 ± 0.00 s  | 1.18 ± 0.02 s  | x 1.02 |
|    app_fibo.rb    | 0.54 ± 0.01 s  | 1.93 ± 0.03 s  | x 3.58 |
|  app_aobench.rb   | 10.29 ± 0.33 s | 22.38 ± 0.19 s | x 2.17 |

## optcarrot benchmark

|    benchmark    |       ruby        |      ruruby      |  rate  |
| :-------------: | :---------------: | :--------------: | :----: |
|    optcarrot    | 43.20 ± 0.78 fps  | 12.80 ± 0.17 fps | x 3.38 |
| optcarrot --opt | 134.38 ± 1.21 fps | 54.55 ± 0.34 fps | x 2.46 |

## memory consumption

|     benchmark     | ruby  | ruruby  |  rate   |
| :---------------: | :---: | :-----: | :-----: |
|     block.rb      | 21.9M |  4.6M   | x 0.21  |
|    for_loop.rb    | 21.9M |  4.6M   | x 0.21  |
| so_mandelbrot.rb  | 22.2M |  5.9M   | x 0.27  |
| app_mandelbrot.rb | 22.2M |  5.5M   | x 0.25  |
|    app_fibo.rb    | 21.9M |  4.7M   | x 0.21  |
|  app_aobench.rb   | 22.6M |  8.0M   | x 0.35  |
|     optcarrot     | 76.8M |  64.2M  | x 0.84  |
|  optcarrot --opt  | 94.1M | 1027.5M | x 10.92 |
