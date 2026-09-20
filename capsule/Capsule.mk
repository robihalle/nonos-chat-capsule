CAPSULE_SLUG := chat
CAPSULE_HANDLE := app.chat
CAPSULE_DOMAIN := local.nonoschat
CAPSULE_DIR := userland/capsule_chat
CAPSULE_BIN_NAME := chat
CAPSULE_FEATURE := nonos-capsule-chat
CAPSULE_NAMESPACE := local.nonoschat.app.chat
CAPSULE_SERVICE_ENDPOINT := service:4990:app.chat
CAPSULE_REPLY_ENDPOINT := reply:4991:endpoint.app.chat.reply
CAPSULE_REQUIRED_CAPS := 0x183d
CAPSULE_KERNEL_MIRROR := src/userspace/capsule_chat
include nonos-mk/capsule.mk
