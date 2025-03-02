# docker-bake.hcl
group "default" {
  targets = ["config_producer", "data_quality", "load_test"]
}

target "config_producer" {
  context = "."
  dockerfile = "Dockerfile"
  target = "config_producer"
  tags = ["config_producer_proto"]
}

target "data_quality" {
  context = "."
  dockerfile = "Dockerfile"
  target = "data_quality"
  tags = ["data_quality_server"]
}

target "load_test" {
  context = "."
  dockerfile = "Dockerfile"
  target = "load_test"
  tags = ["load_test_data_quality_server"]
}

