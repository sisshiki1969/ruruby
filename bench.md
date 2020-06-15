# ruruby benchmark results

## environment

Ruby version: 2.8.0  
CPU: Intel(R) Core(TM) i7-7700 CPU @ 3.60GHz  
OS: Ubuntu 18.04.4 LTS

## execution time

|     benchmark     |       ruby       |     ruruby      |  rate  |
| :---------------: | :--------------: | :-------------: | :----: |
| so_mandelbrot.rb  |  1.80 ± 0.02 s   |  2.95 ± 0.02 s  | x 1.64 |
| app_mandelbrot.rb |  2.25 ± 0.04 s   |  6.42 ± 0.05 s  | x 2.86 |
|      fibo.rb      |  0.47 ± 0.01 s   |  2.12 ± 0.00 s  | x 4.52 |
|     block.rb      |  0.41 ± 0.03 s   |  0.96 ± 0.01 s  | x 2.33 |
|    ao_bench.rb    |  9.25 ± 0.07 s   | 27.28 ± 0.17 s  | x 2.95 |
|     optcarrot     | 50.52 ± 0.69 fps | 7.96 ± 0.13 fps | x 6.35 |

## memory consumption

|     benchmark     | ruby  | ruruby |  rate  |
| :---------------: | :---: | :----: | :----: |
| so_mandelbrot.rb  | 13.8K |  4.9K  | x 0.35 |
| app_mandelbrot.rb | 13.9K | 17.4K  | x 1.25 |
|      fibo.rb      | 13.6K |  4.7K  | x 0.34 |
|     block.rb      | 13.5K |  4.6K  | x 0.34 |
|    ao_bench.rb    | 14.3K | 19.0K  | x 1.33 |
|     optcarrot     | 70.2K | 84.9K  | x 1.21 |
