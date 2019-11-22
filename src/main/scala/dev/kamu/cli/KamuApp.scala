/*
 * Copyright (c) 2018 kamu.dev
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

package dev.kamu.cli

import org.apache.hadoop.conf.Configuration
import org.apache.hadoop.fs.FileSystem
import org.apache.log4j.{Level, LogManager}

class UsageException(message: String = "", cause: Throwable = None.orNull)
    extends RuntimeException(message, cause)

object KamuApp extends App {
  val logger = LogManager.getLogger(getClass.getName)

  val config = KamuConfig()

  val fileSystem = FileSystem.get(new Configuration())
  fileSystem.setWriteChecksum(false)
  fileSystem.setVerifyChecksum(false)

  try {
    val cliArgs = new CliArgs(args)

    LogManager
      .getLogger(getClass.getPackage.getName)
      .setLevel(if (cliArgs.debug()) Level.ALL else cliArgs.logLevel())

    new Kamu(config, fileSystem)
      .run(cliArgs)
  } catch {
    case e: UsageException =>
      logger.error(e.getMessage)
      sys.exit(1)
  }

}
