# Changelog

# 2.0.0

* Removed extra internal boxing
* `Regular::add_boxed_arc()` method was removed
* `Regular`'s `F` generic argument has `Clone` bound and should typically be used with `Arc<T>` now
* `::call_simple()` has implementations not just for callbacks without arguments, but also for callbacks with up to 5  
  arguments (if they are references, can't do more without specialization)

# 1.0.0

* Initial stable release
