# docker-bake.hcl
group "default" {
  targets = ["data_quality", "config_producer"]
}

target "data_quality" {
  context = "."
  dockerfile = "Dockerfile"
  target = "data_quality"
  tags = ["data_quality_server"]
}

target "config_producer" {
  context = "."
  dockerfile = "Dockerfile"
  target = "config_producer"
  tags = ["config_producer_proto"]
}
